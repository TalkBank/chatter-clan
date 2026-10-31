//! CHIP, CLAN's child/adult interaction profile.
//!
//! The implementation uses the typed CHAT tree for surface words and UD
//! `%mor` items. CLAN's legacy morphology format is not represented by Chatter
//! v0.33.0, so that part of the original implementation is an explicit
//! divergence rather than a string-parsing fallback.

mod config;
mod interaction;
mod output;
#[cfg(test)]
mod tests;
mod word;

use std::collections::VecDeque;

use interaction::{Code, Column, Outcome, Params, Role, Tallies, Utter, single_interaction};
use talkbank_model::{DependentTier, Utterance, WriteChat};

use crate::framework::{AnalysisCommand, FileContext};

pub use config::{
    ChipCaseMode, ChipConfig, ChipConfigError, ColumnScoring, MinIndex, MinWords, Scoring,
    SpeakerCode, Substitution, UtteranceWindow, WordSource,
};
pub use output::{ChipFileReport, ChipFileStatus, ChipMatrix, ChipResult, MatrixRow};
use word::{MisalignedMor, utterance_words};

/// State accumulated across a complete CHIP invocation.
#[derive(Debug, Default)]
pub struct ChipState {
    files: Vec<ChipFileReport>,
    active_file: Option<ActiveFile>,
}

#[derive(Debug)]
struct ActiveFile {
    filename: String,
    transcript: String,
    window: VecDeque<Utter>,
    tallies: Tallies,
    worded_utterances: usize,
    failure: Option<MisalignedMor>,
}

impl ActiveFile {
    fn new(context: &FileContext<'_>) -> Self {
        Self {
            filename: context.path.file_name().map_or_else(
                || context.filename.to_owned(),
                |name| name.to_string_lossy().into(),
            ),
            transcript: String::new(),
            window: VecDeque::new(),
            tallies: Tallies::default(),
            worded_utterances: 0,
            failure: None,
        }
    }

    fn append_line(&mut self, line: &str) {
        self.transcript.push_str(line);
        self.transcript.push('\n');
    }
}

/// CHIP scoring command.
#[derive(Debug, Clone, Default)]
pub struct ChipCommand {
    config: ChipConfig,
}

impl ChipCommand {
    /// Build CHIP with validated command-line options.
    pub fn new(config: ChipConfig) -> Self {
        Self { config }
    }

    fn role_for(&self, speaker: &str) -> Role {
        if self
            .config
            .children()
            .iter()
            .any(|code| code.as_str() == speaker)
        {
            Role::Child
        } else if self
            .config
            .adults()
            .iter()
            .any(|code| code.as_str() == speaker)
        {
            Role::Adult
        } else {
            Role::Other
        }
    }

    fn score_pair(
        &self,
        tallies: &mut Tallies,
        source: &Utter,
        response: &Utter,
        column: Column,
        distance: usize,
        tier: &str,
    ) -> Option<String> {
        let outcome = single_interaction(
            &source.words,
            &response.words,
            Params {
                subst: self.config.source.substitution(),
                min_words: self.config.min_words,
                min_index: self.config.min_index,
            },
            tallies.column_mut(column),
        );
        let Outcome::Scored { rep_index, codes } = outcome else {
            return None;
        };
        if rep_index > self.config.min_index.get() {
            tallies.column_mut(column).dist += distance as u64;
        }

        let mut code = codes.iter().map(ToString::to_string).collect::<String>();
        if rep_index > self.config.min_index.get() {
            code.push_str(&Code::Dist(distance).to_string());
        }
        code.push_str(&Code::Rep(rep_index).to_string());
        Some(format!("{tier}:\t{}", code.trim_start()))
    }

    fn process_scoring(&self, file: &mut ActiveFile, current: &Utter) -> Vec<String> {
        let mut tiers = Vec::new();
        if current.words.is_empty() {
            return tiers;
        }

        if self.config.scoring.self_rep == Scoring::Enabled
            && matches!(current.role, Role::Adult | Role::Child)
            && let Some((distance, previous)) =
                file.window.iter().enumerate().find(|(_, previous)| {
                    previous.role == current.role
                        && previous.speaker == current.speaker
                        && !previous.words.is_empty()
                })
        {
            let target = match current.role {
                Role::Adult => Some((Column::AdultSelf, "%asr")),
                Role::Child => Some((Column::ChildSelf, "%csr")),
                Role::Other => None,
            };
            if let Some((column, tier)) = target
                && let Some(code) = self.score_pair(
                    &mut file.tallies,
                    previous,
                    current,
                    column,
                    distance + 1,
                    tier,
                )
            {
                tiers.push(code);
            }
        }

        let partner_role = match current.role {
            Role::Adult if self.config.scoring.adult == Scoring::Enabled => Some(Role::Child),
            Role::Child if self.config.scoring.child == Scoring::Enabled => Some(Role::Adult),
            Role::Adult | Role::Child | Role::Other => None,
        };
        if let Some(partner_role) = partner_role
            && let Some((distance, previous)) =
                file.window.iter().enumerate().find(|(_, previous)| {
                    previous.role == partner_role && !previous.words.is_empty()
                })
        {
            let target = match current.role {
                Role::Adult => Some((Column::Adult, "%adu")),
                Role::Child => Some((Column::Child, "%chi")),
                Role::Other => None,
            };
            if let Some((column, tier)) = target
                && let Some(code) = self.score_pair(
                    &mut file.tallies,
                    previous,
                    current,
                    column,
                    distance + 1,
                    tier,
                )
            {
                tiers.push(code);
            }
        }
        tiers
    }
}

impl AnalysisCommand for ChipCommand {
    type Config = ChipConfig;
    type State = ChipState;
    type Output = ChipResult;

    fn process_utterance(
        &self,
        utterance: &Utterance,
        context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        let file = state
            .active_file
            .get_or_insert_with(|| ActiveFile::new(context));
        if file.failure.is_some() {
            return;
        }

        let words = match utterance_words(utterance, &self.config) {
            Ok(words) => words,
            Err(error) => {
                file.failure = Some(error);
                return;
            }
        };

        if !words.is_empty() {
            file.worded_utterances += 1;
            match self.role_for(utterance.main.speaker.as_str()) {
                Role::Adult => file.tallies.adult.utters += 1,
                Role::Child => file.tallies.child.utters += 1,
                Role::Other => {}
            }
        }

        let current = Utter {
            speaker: utterance.main.speaker.as_str().to_owned(),
            role: self.role_for(utterance.main.speaker.as_str()),
            words,
        };
        let generated_tiers = self.process_scoring(file, &current);

        file.append_line(&utterance.main.to_chat_string());
        for dependent in &utterance.dependent_tiers {
            if matches!(&dependent.tier, DependentTier::Mor(_)) {
                file.append_line(&dependent.tier.to_chat_string());
            }
        }
        for tier in generated_tiers {
            file.append_line(&tier);
        }

        file.window.push_front(current);
        file.window.truncate(self.config.window.capacity());
    }

    fn end_file(&self, _context: &FileContext<'_>, state: &mut Self::State) {
        let Some(file) = state.active_file.take() else {
            return;
        };
        let status = match file.failure {
            Some(error) => ChipFileStatus::Rejected(error.to_string()),
            None if file.worded_utterances == 0 => ChipFileStatus::NoScorableUtterances,
            None => ChipFileStatus::Scored(ChipMatrix::build(&file.tallies, &self.config)),
        };
        state.files.push(ChipFileReport {
            filename: file.filename,
            transcript: file.transcript,
            status,
        });
    }

    fn finalize(&self, state: Self::State) -> Self::Output {
        ChipResult { files: state.files }
    }
}
