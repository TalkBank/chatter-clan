//! Typed service-layer analysis requests and errors.

use std::path::PathBuf;

use thiserror::Error;

use crate::commands::chains::ChainsConfig;
use crate::commands::codes::CodesConfig;
use crate::commands::combo::ComboConfig;
use crate::commands::cooccur::CooccurConfig;
use crate::commands::corelex::CorelexConfig;
use crate::commands::dist::DistConfig;
use crate::commands::dss::DssConfig;
use crate::commands::eval::EvalConfig;
use crate::commands::flucalc::FlucalcConfig;
use crate::commands::freq::FreqConfig;
use crate::commands::freqpos::FreqposConfig;
use crate::commands::ipsyn::IpsynConfig;
use crate::commands::keymap::KeymapConfig;
use crate::commands::kideval::KidevalConfig;
use crate::commands::kwal::KwalConfig;
use crate::commands::maxwd::MaxwdConfig;
use crate::commands::mlt::MltConfig;
use crate::commands::mlu::MluConfig;
use crate::commands::mortable::MortableConfig;
use crate::commands::rely::RelyConfig;
use crate::commands::script::ScriptConfig;
use crate::commands::sugar::SugarConfig;
use crate::commands::trnfix::TrnfixConfig;
use crate::commands::uniq::UniqConfig;
use crate::commands::vocd::VocdConfig;
use crate::commands::wdsize::WdsizeConfig;
use crate::framework::{RunnerError, TransformError};

/// Typed, library-owned request for a CLAN analysis command.
///
/// This enum is the stable integration boundary for higher-level consumers such
/// as the CLI and LSP. It keeps command-specific configuration typed without
/// forcing those outer layers to import and execute each command type directly.
#[derive(Debug)]
pub enum AnalysisRequest {
    /// `freq`
    Freq(FreqConfig),
    /// `mlu`
    Mlu(MluConfig),
    /// `mlt`
    Mlt(MltConfig),
    /// `wdlen`
    Wdlen,
    /// `wdsize`
    Wdsize(WdsizeConfig),
    /// `maxwd`
    Maxwd(MaxwdConfig),
    /// `freqpos`
    Freqpos(FreqposConfig),
    /// `timedur`
    Timedur,
    /// `kwal`
    Kwal(KwalConfig),
    /// `gemlist`
    Gemlist,
    /// `combo`
    Combo(ComboConfig),
    /// `cooccur`
    Cooccur(CooccurConfig),
    /// `dist`
    Dist(DistConfig),
    /// `chip`
    Chip,
    /// `phonfreq`
    Phonfreq,
    /// `modrep`
    Modrep,
    /// `vocd`
    Vocd(VocdConfig),
    /// `codes`
    Codes(CodesConfig),
    /// `chains`
    Chains(ChainsConfig),
    /// `complexity`
    Complexity,
    /// `corelex`
    Corelex(CorelexConfig),
    /// `dss`
    Dss(DssConfig),
    /// `eval`
    Eval(EvalConfig),
    /// `flucalc`
    Flucalc(FlucalcConfig),
    /// `ipsyn`
    Ipsyn(IpsynConfig),
    /// `keymap`
    Keymap(KeymapConfig),
    /// `kideval`
    Kideval(KidevalConfig),
    /// `mortable`
    Mortable(MortableConfig),
    /// `script`
    Script(ScriptConfig),
    /// `sugar`
    Sugar(SugarConfig),
    /// `trnfix`
    Trnfix(TrnfixConfig),
    /// `uniq`
    Uniq(UniqConfig),
}

/// Built analysis plan after library-owned defaults and validation are applied.
#[derive(Debug)]
pub enum AnalysisPlan {
    /// Standard request executed through `AnalysisService`.
    Service(AnalysisRequest),
    /// `rely` still uses an explicit two-file execution path.
    Rely(RelyRequest),
}

/// Typed request for `rely`.
#[derive(Debug)]
pub struct RelyRequest {
    /// Parsed secondary file path.
    pub secondary_file: PathBuf,
    /// Validated RELY configuration.
    pub config: RelyConfig,
}

impl AnalysisRequest {
    /// Validate and construct a `kwal` request. The caller assembles the
    /// `KwalConfig` (likely from a `KwalOptions`); this function's only
    /// job is the non-empty-keywords check.
    pub fn kwal(config: KwalConfig) -> Result<Self, AnalysisServiceError> {
        if config.keywords.is_empty() {
            return Err(AnalysisServiceError::InvalidRequest(
                "kwal requires at least one keyword".to_owned(),
            ));
        }
        Ok(Self::Kwal(config))
    }

    /// Validate and construct a `keymap` request.
    pub fn keymap(
        keywords: Vec<crate::framework::KeywordPattern>,
        tier: crate::framework::TierKind,
    ) -> Result<Self, AnalysisServiceError> {
        if keywords.is_empty() {
            return Err(AnalysisServiceError::InvalidRequest(
                "keymap requires at least one keyword".to_owned(),
            ));
        }

        Ok(Self::Keymap(KeymapConfig { keywords, tier }))
    }
}

/// Error from the high-level analysis service boundary.
#[derive(Debug, Error)]
pub enum AnalysisServiceError {
    /// Invalid request shape or unsupported option combination.
    #[error("{0}")]
    InvalidRequest(String),
    /// Underlying transform failure used by non-runner commands such as `rely`.
    #[error(transparent)]
    Transform(#[from] TransformError),
    /// Underlying runner failure.
    #[error(transparent)]
    Runner(#[from] RunnerError),
}
