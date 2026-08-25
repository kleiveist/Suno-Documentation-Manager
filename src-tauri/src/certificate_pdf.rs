use crate::certificate::{localized_certificate_label, localized_certificate_paragraph};
use crate::error::{AppError, Result};
#[cfg(test)]
use crate::model::SunoLyricsContentSource;
use crate::model::{
    AudioScreeningExternalRecord, AudioScreeningMode, AudioScreeningProviderStatus,
    AudioScreeningStatus, BlockingDeviation, CertificateLanguage, CertificateRenderOptions,
    DocumentationAnswer, EvidenceItem, EvidenceRole, ExternalTimestampStatus, FactOrigin,
    FinalizationTimestampSnapshot, Profile, StepState, StepStatus, SunoContentClassification,
    TimestampProviderConfigurationStatus, TimestampProviderMetadata, TimestampQualificationStatus,
    TrackAutomation, TrackFields, TrackRecord, TrustedListValidationStatus, VocalIntent,
};
use crate::workflow;
use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use image::imageops::FilterType;
use lopdf::{
    content::Content as LoContent, Dictionary as LoDictionary, Document as LoDocument,
    LoadOptions as LoLoadOptions, Object as LoObject, Stream as LoStream,
    StringFormat as LoStringFormat,
};
use printpdf::{
    BuiltinFont, Cmyk, Color, DictItem, ExternalStream, ExternalXObject, FontId, Line, LinePoint,
    Mm, Op, PageAnnotId, ParsedFont, PdfConformance, PdfDocument, PdfFont, PdfFontHandle, PdfPage,
    PdfParseErrorSeverity, PdfParseOptions, PdfSaveOptions, Point, Pt, Px, TextItem, XObject,
    XObjectId, XObjectTransform,
};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

mod artwork;
mod layout;
mod pdf_generation;
mod pdf_validation;
mod render_appendix;
mod render_overview;
mod render_rights;
mod render_track;
mod render_workflow;
mod snapshot_validation;
mod table;
mod timestamp_addendum;
mod timestamp_rows;
mod view_model;

pub use artwork::*;
use layout::*;
pub use pdf_generation::*;
pub use pdf_validation::*;
use render_appendix::*;
use render_overview::*;
use render_rights::*;
use render_track::*;
use render_workflow::*;
use snapshot_validation::*;
use table::*;
pub use timestamp_addendum::*;
use timestamp_rows::*;
pub use view_model::*;

#[cfg(test)]
mod tests;
