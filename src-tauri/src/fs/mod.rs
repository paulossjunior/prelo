//! Camada que toca o disco do projeto (specs/003-editor-local/plan.md, `fs/`): a regra de
//! escopo de arquivo (RF-08, contracts/file-access.md), o `PathAccessModel` que o World usa e a
//! gravação dos arquivos abertos (`save`), a criação de arquivo e pasta (`create`) e o
//! observador de mudanças externas (`watch`), e a cópia de arquivo de fora para dentro do
//! projeto (`copy_into`, D-13).

pub mod access_model;
pub mod copy_into;
pub mod create;
pub mod save;
pub mod scope;
pub mod watch;
