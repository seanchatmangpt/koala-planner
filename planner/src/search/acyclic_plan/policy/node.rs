use super::HTN;
use std::collections::HashSet;
use std::rc::Rc;
#[derive(Debug)]
pub struct PolicyNode {
    pub state: HashSet<String>,
    pub tn: Rc<HTN>,
}
