use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RiskAssessmentConfig {
    pub change_impact_assessment: ChangeImpactAssessmentConfig,
    pub change_risk_assessment: Option<ChangeRiskAssessmentConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChangeImpactAssessmentConfig {
    pub security_controls_impact: Option<String>,
    pub performance_impact: Option<String>,
    pub availability_impact: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChangeRiskAssessmentConfig {
    // Placeholder for future expansion
}
