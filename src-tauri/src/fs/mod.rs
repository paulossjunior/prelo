//! Camada que toca o disco do projeto (specs/003-editor-local/plan.md, `fs/`): a regra de
//! escopo de arquivo (RF-08, contracts/file-access.md) e o `PathAccessModel` que o World usa.

pub mod access_model;
pub mod scope;
