//! Camada que toca o disco do projeto (specs/003-editor-local/plan.md, `fs/`): a regra de
//! escopo de arquivo (RF-08, contracts/file-access.md), o `PathAccessModel` que o World usa e a
//! gravação dos arquivos abertos (`save`).

pub mod access_model;
pub mod save;
pub mod scope;
