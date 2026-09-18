pub struct Project {
    pub name: Option<String>,
    pub directory: Option<String>,
}

#[derive(PartialEq)]
pub enum ProjectStep {
    Name,
    Directory,
    Confirm,
}
