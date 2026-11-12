//! Port of MatchValidationMessage.ts

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ok = 0,
    Error = 1,
    Warning = 2,
    Info = 3,
    HiddenError = 4,
}

impl Status {
    pub fn from_num(n: f64) -> Status {
        match n as i64 {
            1 => Status::Error,
            2 => Status::Warning,
            3 => Status::Info,
            4 => Status::HiddenError,
            _ => Status::Ok,
        }
    }
}

/// MatchValidationType enum. The string is what's stored in files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VType {
    LowTotalTuh,
    InvalidTotalTuh,
    MissingTeams,
    MissingTotalPoints,
    TeamPlayingItself,
    InvalidTeamScore,
    TeamsNotInSamePool,
    PlayerTuhInvalid,
    TieGame,
    NoTossupsHeard,
    FewerThanExpectedTUH,
    MoreThanAllowedTUH,
    PlayerAnswerCountInvalid,
    PlayerHasTooManyBuzzes,
    MatchHasTooConvertedTU,
    TeamHasTooManyBuzzes,
    NegativeBonusPoints,
    BonusPointsTooHigh,
    BonusDivisorMismatch,
    DoubleForfeit,
    InvalidOvertimeTuh,
    OtButRegScoreNotTied,
    RegulationTuhNotStandard,
    TotalOtBuzzesExceedsTuh,
    OtTuhLessThanMinimum,
    TeamAlreadyPlayedInRound,
    InvalidBouncebackPoints,
    BouncebackConvOver100,
    TotalScoreAndTuPtsMismatch,
    ImportTwoMatchesSameTeam,
    TuPlusLtngNotEqualTotal,
    LightningDivisorMismatch,
    BouncebackDivisorMismatch,
    OvertimeTuhTooHigh,
    ManualBonusMathMismatch,
    /// A type string we don't recognize (kept so suppression state survives)
    Other,
}

impl VType {
    /// The string stored in files (MatchValidationType value)
    pub fn as_str(&self) -> &'static str {
        use VType::*;
        match self {
            LowTotalTuh => "LowTotalTuh",
            InvalidTotalTuh => "InvalidTotalTuh",
            MissingTeams => "MissingTeams",
            MissingTotalPoints => "MissingTotalPoints",
            TeamPlayingItself => "TeamPlayingItself",
            InvalidTeamScore => "InvalidTeamScore",
            TeamsNotInSamePool => "TeamsNotInSamePool",
            PlayerTuhInvalid => "PlayherTuhInvalid",
            TieGame => "TieGame",
            NoTossupsHeard => "NoTossupsHeard",
            FewerThanExpectedTUH => "FewerThanExpectedTUH",
            MoreThanAllowedTUH => "MoreThanAllowedTUH",
            PlayerAnswerCountInvalid => "PlayerAnswerCountInvalid",
            PlayerHasTooManyBuzzes => "PlayerHasTooManyBuzzes",
            MatchHasTooConvertedTU => "MatchHasTooConvertedTU",
            TeamHasTooManyBuzzes => "TeamHasTooManyBuzzes",
            NegativeBonusPoints => "NegativeBonusPoints",
            BonusPointsTooHigh => "BonusPointsTooHigh",
            BonusDivisorMismatch => "BonusDivisorMismatch",
            DoubleForfeit => "DoubleForfeit",
            InvalidOvertimeTuh => "InvalidOvertimeTuh",
            OtButRegScoreNotTied => "OtButRegScoreNotTied",
            RegulationTuhNotStandard => "RegulationTuhNotStandard",
            TotalOtBuzzesExceedsTuh => "TotalOtBuzzesExceedsTuh",
            OtTuhLessThanMinimum => "OtTuhLessThanMinimum",
            TeamAlreadyPlayedInRound => "TeamAlreadyPlayedInRound",
            InvalidBouncebackPoints => "InvalidBouncebackPoints",
            TotalScoreAndTuPtsMismatch => "TotalScoreAndTuPtsMismatch",
            ImportTwoMatchesSameTeam => "ImportTwoMatchesSameTeam",
            TuPlusLtngNotEqualTotal => "TuPlusLtngNotEqualTotal",
            LightningDivisorMismatch => "LightningDivisorMismatch",
            BouncebackDivisorMismatch => "BouncebackDivisorMismatch",
            OvertimeTuhTooHigh => "OvertimeTuhTooHigh",
            ManualBonusMathMismatch => "ManualBonusMathMismatch",
            BouncebackConvOver100 => "BouncebackConvOver100",
            Other => "",
        }
    }

    pub fn from_str(s: &str) -> VType {
        use VType::*;
        match s {
            "LowTotalTuh" => LowTotalTuh,
            "InvalidTotalTuh" => InvalidTotalTuh,
            "MissingTeams" => MissingTeams,
            "MissingTotalPoints" => MissingTotalPoints,
            "TeamPlayingItself" => TeamPlayingItself,
            "InvalidTeamScore" => InvalidTeamScore,
            "TeamsNotInSamePool" => TeamsNotInSamePool,
            "PlayherTuhInvalid" => PlayerTuhInvalid,
            "TieGame" => TieGame,
            "NoTossupsHeard" => NoTossupsHeard,
            "FewerThanExpectedTUH" => FewerThanExpectedTUH,
            "MoreThanAllowedTUH" => MoreThanAllowedTUH,
            "PlayerAnswerCountInvalid" => PlayerAnswerCountInvalid,
            "PlayerHasTooManyBuzzes" => PlayerHasTooManyBuzzes,
            "MatchHasTooConvertedTU" => MatchHasTooConvertedTU,
            "TeamHasTooManyBuzzes" => TeamHasTooManyBuzzes,
            "NegativeBonusPoints" => NegativeBonusPoints,
            "BonusPointsTooHigh" => BonusPointsTooHigh,
            "BonusDivisorMismatch" => BonusDivisorMismatch,
            "DoubleForfeit" => DoubleForfeit,
            "InvalidOvertimeTuh" => InvalidOvertimeTuh,
            "OtButRegScoreNotTied" => OtButRegScoreNotTied,
            "RegulationTuhNotStandard" => RegulationTuhNotStandard,
            "TotalOtBuzzesExceedsTuh" => TotalOtBuzzesExceedsTuh,
            "OtTuhLessThanMinimum" => OtTuhLessThanMinimum,
            "TeamAlreadyPlayedInRound" => TeamAlreadyPlayedInRound,
            "InvalidBouncebackPoints" => InvalidBouncebackPoints,
            "BouncebackConvOver100" => BouncebackConvOver100,
            "TotalScoreAndTuPtsMismatch" => TotalScoreAndTuPtsMismatch,
            "ImportTwoMatchesSameTeam" => ImportTwoMatchesSameTeam,
            "TuPlusLtngNotEqualTotal" => TuPlusLtngNotEqualTotal,
            "LightningDivisorMismatch" => LightningDivisorMismatch,
            "BouncebackDivisorMismatch" => BouncebackDivisorMismatch,
            "OvertimeTuhTooHigh" => OvertimeTuhTooHigh,
            "ManualBonusMathMismatch" => ManualBonusMathMismatch,
            _ => Other,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Msg {
    pub status: Status,
    pub message: String,
    pub suppressable: bool,
    pub is_suppressed: bool,
    pub vtype: VType,
    /// original type string, for unknown types
    pub type_str: String,
}

impl Msg {
    /// The `type` written to files: the known type's name, or the unrecognized string read from the file
    pub fn type_string(&self) -> &str {
        if self.vtype == VType::Other {
            &self.type_str
        } else {
            self.vtype.as_str()
        }
    }
    pub fn new(vtype: VType) -> Msg {
        Msg { status: Status::Ok, message: String::new(), suppressable: false, is_suppressed: false, vtype, type_str: String::new() }
    }
    pub fn is_error(&self) -> bool {
        self.status == Status::Error || self.status == Status::HiddenError
    }
    pub fn set_ok(&mut self) {
        self.status = Status::Ok;
        self.message.clear();
    }
    pub fn set_error(&mut self, msg: String) {
        self.status = Status::Error;
        self.message = msg;
    }
}

#[derive(Clone, Debug, Default)]
pub struct Collection {
    pub validators: Vec<Msg>,
}

impl Collection {
    pub fn error_messages(&self, ignore_hidden: bool) -> Vec<String> {
        self.validators
            .iter()
            .filter(|v| {
                if v.is_suppressed {
                    return false;
                }
                if v.status == Status::Error {
                    return true;
                }
                if ignore_hidden {
                    return false;
                }
                v.status == Status::HiddenError
            })
            .map(|v| v.message.clone())
            .collect()
    }

    pub fn warning_messages(&self) -> Vec<String> {
        self.validators
            .iter()
            .filter(|v| v.status == Status::Warning && !v.is_suppressed)
            .map(|v| v.message.clone())
            .collect()
    }

    fn find(&self, t: VType) -> Option<&Msg> {
        self.validators.iter().find(|v| v.vtype == t)
    }

    pub fn has(&self, t: VType) -> bool {
        self.find(t).is_some()
    }

    pub fn add(&mut self, t: VType, status: Status, message: String) {
        self.add_full(t, status, message, None, false, String::new());
    }

    pub fn add_full(
        &mut self,
        t: VType,
        status: Status,
        message: String,
        suppressable: Option<bool>,
        is_suppressed: bool,
        type_str: String,
    ) {
        let same = |v: &Msg| v.vtype == t && (t != VType::Other || v.type_str == type_str);
        // don't "unsuppress" things just because we noticed the issue still exists
        if self.validators.iter().find(|v| same(v)).is_some_and(|m| m.is_suppressed) {
            return;
        }
        self.validators.retain(|v| !same(v));
        let suppressable = suppressable.unwrap_or(status == Status::Warning);
        self.validators.push(Msg { status, message, suppressable, is_suppressed, vtype: t, type_str });
    }

    pub fn clear(&mut self, t: VType) {
        self.validators.retain(|v| v.vtype != t);
    }
}
