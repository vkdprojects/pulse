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
    UpdateMaster(String), // host IP to pre-fill
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
    RenameMaster(String),
    RenameAgent(String, String), // agent_id, current_name
    ConfirmDelete(String),
}
