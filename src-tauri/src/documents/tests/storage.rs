use super::super::*;
use super::support::*;
use crate::error::AppError;
use std::fs;

#[test]
fn adopt_existing_false_leaves_unmanaged_sentinel_unchanged() {
    let workspace = tempfile::tempdir().expect("temporary track root");
    let sentinel = write_adoption_sentinel(workspace.path());
    let (track, profile, evidence) = fixture_input();

    let result = generate(workspace.path(), &track, &profile, &evidence, &[], false);

    assert!(matches!(result, Err(AppError::AdoptionRequired(_))));
    assert_eq!(
        fs::read(sentinel).expect("read sentinel"),
        ADOPTION_SENTINEL
    );
    assert!(!workspace.path().join("02_SUNO/suno_project.txt").exists());
}

#[test]
fn adopt_existing_true_archives_exact_bytes_before_managed_replacement() {
    let workspace = tempfile::tempdir().expect("temporary track root");
    let sentinel = write_adoption_sentinel(workspace.path());
    let (track, profile, evidence) = fixture_input();
    let expected_readme = render(&track, &profile, &evidence, &[])["03_DOCUMENTATION/README.md"]
        .as_bytes()
        .to_vec();

    generate(workspace.path(), &track, &profile, &evidence, &[], true)
        .expect("adopt unmanaged document");

    let adoption_roots = fs::read_dir(workspace.path().join(".archive/adoptions"))
        .expect("read adoption archive")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("collect adoption archive entries");
    assert_eq!(adoption_roots.len(), 1);
    assert!(adoption_roots[0]
        .file_type()
        .expect("adoption entry type")
        .is_dir());
    let backup = adoption_roots[0].path().join("03_DOCUMENTATION/README.md");
    assert_eq!(
        fs::read(backup).expect("read archived sentinel"),
        ADOPTION_SENTINEL
    );
    assert_eq!(
        fs::read(sentinel).expect("read managed replacement"),
        expected_readme
    );
}

#[test]
fn forced_adoption_backup_failure_leaves_original_unchanged() {
    let workspace = tempfile::tempdir().expect("temporary track root");
    let sentinel = write_adoption_sentinel(workspace.path());
    fs::write(
        workspace.path().join(".archive"),
        b"block archive directory creation",
    )
    .expect("create forced archive failure");
    let (track, profile, evidence) = fixture_input();

    let result = generate(workspace.path(), &track, &profile, &evidence, &[], true);

    assert!(
        matches!(&result, Err(AppError::Io { .. })),
        "forced backup failure returned an unexpected result: {result:?}"
    );
    assert_eq!(
        fs::read(sentinel).expect("read sentinel"),
        ADOPTION_SENTINEL
    );
    assert!(!workspace.path().join("02_SUNO/suno_project.txt").exists());
}

#[test]
fn preview_requires_the_marker_at_the_exact_header() {
    let workspace = tempfile::tempdir().expect("temporary track root");
    let path = workspace.path().join("03_DOCUMENTATION/README.md");
    fs::create_dir_all(path.parent().expect("document parent")).expect("create document parent");
    fs::write(
        &path,
        format!("# Legacy document\n\nThis mentions {MANAGED_MARKER}, but is not managed.\n"),
    )
    .expect("write legacy document");

    let result = preview(workspace.path()).expect("preview documents");
    assert!(result.adoption_required);
    assert_eq!(result.collisions, vec!["03_DOCUMENTATION/README.md"]);

    fs::write(&path, format!("{MARKDOWN_MARKER_HEADER}# Managed\n"))
        .expect("write managed document");
    let result = preview(workspace.path()).expect("preview managed documents");
    assert!(!result.adoption_required);
    assert!(result.collisions.is_empty());
}
