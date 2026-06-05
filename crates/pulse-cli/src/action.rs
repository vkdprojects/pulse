use crate::client::AgentOverview;

#[derive(Debug, Clone)]
pub enum Action {
    None,
    Quit,
    NavigateHome,
    NavigateFleet(String),
    NavigateDetail(String),
    DataLoaded(Vec<AgentOverview>),
    RefreshData,
    ShowModal(ModalAction),
    CloseModal,
}

#[derive(Debug, Clone)]
pub enum ModalAction {
    AddMaster,
    ConfirmDelete(String),
}
