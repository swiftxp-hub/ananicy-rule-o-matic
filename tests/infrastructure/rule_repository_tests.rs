use ananicy_rule_o_matic::infrastructure::rule_repository::RuleRepository;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_load_rules_simple()
{
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.rules");
    let content = r#"{"name": "test_process", "type": "Game"}"#;

    fs::write(file_path, content).unwrap();

    let rule_repository = RuleRepository::new(vec![dir.path().to_path_buf()]);
    let rules = rule_repository.load_all().unwrap();

    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].data.name.as_deref(), Some("test_process"));
    assert_eq!(rules[0].data.rule_type.as_deref(), Some("Game"));
    assert_eq!(rules[0].source_file, dir.path().join("test.rules"));
}

#[test]
fn test_load_rules_multiple_files()
{
    let dir = tempdir().unwrap();

    let file1 = dir.path().join("01_test.rules");
    let content1 = r#"{"name": "proc1", "nice": -5}"#;
    fs::write(file1, content1).unwrap();

    let file2 = dir.path().join("02_test.rules");
    let content2 = r#"{"name": "proc2", "nice": 10}"#;
    fs::write(file2, content2).unwrap();

    let rule_repository = RuleRepository::new(vec![dir.path().to_path_buf()]);
    let rules = rule_repository.load_all().unwrap();

    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].data.name.as_deref(), Some("proc1"));
    assert_eq!(rules[1].data.name.as_deref(), Some("proc2"));
}

#[test]
fn test_load_rules_multiple_directories()
{
    let dir1 = tempdir().unwrap();
    let dir2 = tempdir().unwrap();

    let file1 = dir1.path().join("test1.rules");
    fs::write(file1, r#"{"name": "proc1"}"#).unwrap();

    let file2 = dir2.path().join("test2.rules");
    fs::write(file2, r#"{"name": "proc2"}"#).unwrap();

    let rule_repository = RuleRepository::new(vec![dir1.path().to_path_buf(), dir2.path().to_path_buf()]);
    let rules = rule_repository.load_all().unwrap();

    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].data.name.as_deref(), Some("proc1"));
    assert_eq!(rules[1].data.name.as_deref(), Some("proc2"));
}

#[test]
fn test_ignore_non_rule_files()
{
    let dir = tempdir().unwrap();

    let rule_file = dir.path().join("test.rules");
    fs::write(rule_file, r#"{"name": "valid"}"#).unwrap();

    let txt_file = dir.path().join("readme.txt");
    fs::write(txt_file, "This is not a rule file").unwrap();

    let rule_repository = RuleRepository::new(vec![dir.path().to_path_buf()]);
    let rules = rule_repository.load_all().unwrap();

    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].data.name.as_deref(), Some("valid"));
}

#[test]
fn test_comments_parsing()
{
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("comments.rules");
    let content = r#"
# This is a comment
{"name": "proc1"}

# Another comment
# spanning two lines
{"name": "proc2"}
"#;
    fs::write(file_path, content).unwrap();

    let rule_repository = RuleRepository::new(vec![dir.path().to_path_buf()]);
    let rules = rule_repository.load_all().unwrap();

    assert_eq!(rules.len(), 2);

    assert_eq!(rules[0].data.name.as_deref(), Some("proc1"));
    assert_eq!(rules[0].context_comment.as_deref(), Some("# This is a comment"));

    assert_eq!(rules[1].data.name.as_deref(), Some("proc2"));
    assert_eq!(
        rules[1].context_comment.as_deref(),
        Some(
            "# Another comment
# spanning two lines"
        )
    );
}

#[test]
fn test_block_comments_parsing()
{
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("block_comments.rules");
    let content = r#"
# Common comment
{"name": "proc1"}
{"name": "proc2"}

# Separate comment
{"name": "proc3"}
"#;
    fs::write(file_path, content).unwrap();

    let rule_repository = RuleRepository::new(vec![dir.path().to_path_buf()]);
    let rules = rule_repository.load_all().unwrap();

    assert_eq!(rules.len(), 3);

    assert_eq!(rules[0].data.name.as_deref(), Some("proc1"));
    assert_eq!(rules[0].context_comment.as_deref(), Some("# Common comment"));

    assert_eq!(rules[1].data.name.as_deref(), Some("proc2"));
    assert_eq!(rules[1].context_comment.as_deref(), Some("# Common comment"));

    assert_eq!(rules[2].data.name.as_deref(), Some("proc3"));
    assert_eq!(rules[2].context_comment.as_deref(), Some("# Separate comment"));
}

#[test]
fn test_invalid_json_lines_skipped()
{
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("invalid.rules");
    let content = r#"
{"name": "valid1"}
{invalid json}
{"name": "valid2"}
not even json
"#;
    fs::write(file_path, content).unwrap();

    let rule_repository = RuleRepository::new(vec![dir.path().to_path_buf()]);
    let rules = rule_repository.load_all().unwrap();

    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].data.name.as_deref(), Some("valid1"));
    assert_eq!(rules[1].data.name.as_deref(), Some("valid2"));
}

#[test]
fn test_non_existent_directory()
{
    let non_existent = PathBuf::from("/path/to/nowhere/hopefully");
    let rule_repository = RuleRepository::new(vec![non_existent]);
    let rules = rule_repository.load_all().unwrap();

    assert!(rules.is_empty());
}

#[test]
fn test_empty_directory()
{
    let dir = tempdir().unwrap();
    let rule_repository = RuleRepository::new(vec![dir.path().to_path_buf()]);
    let rules = rule_repository.load_all().unwrap();

    assert!(rules.is_empty());
}
