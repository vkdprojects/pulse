use crate::client::AgentOverview;
use crate::screens::provision::ProvisionTarget;

#[derive(Debug, Clone)]
pub enum Action {
    None,
    Quit,
    NavigateHome,
    NavigateFleet(String),
    NavigateDetail(String),
    NavigateExec(String, String), // agent_id, hostname
    NavigateUpdate,
    NavigateProvision(ProvisionTarget),
    DataLoaded(Vec<AgentOverview>),
    RefreshData,
    ShowModal(ModalAction),
    CloseModal,
    RunExec,
}

#[derive(Debug, Clone)]
pub enum ModalAction {
    AddMaster,
    ConfirmDelete(String),
}
