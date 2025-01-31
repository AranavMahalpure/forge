use generate::Generate;
use gh_workflow_tailcall::*;
use indexmap::indexmap;
use serde_json::json;

#[test]
fn generate() {
    let mut workflow = StandardWorkflow::default()
        .auto_fix(true)
        .add_setup(Step::run("sudo apt-get install -y libsqlite3-dev"))
        .to_ci_workflow()
        .add_env(("FORGE_KEY", "${{secrets.FORGE_KEY}}"));

    // Set up the build matrix for all platforms
    let matrix = json!({
        "include": [
            {
                "os": "ubuntu-latest",
                "target": "x86_64-unknown-linux-gnu",
                "binary_name": "forge-x86_64-unknown-linux-gnu",
                "binary_path": "target/x86_64-unknown-linux-gnu/release/forge_main"
            },
            {
                "os": "macos-latest",
                "target": "x86_64-apple-darwin",
                "binary_name": "forge-x86_64-apple-darwin",
                "binary_path": "target/x86_64-apple-darwin/release/forge_main"
            },
            {
                "os": "macos-latest",
                "target": "aarch64-apple-darwin",
                "binary_name": "forge-aarch64-apple-darwin",
                "binary_path": "target/aarch64-apple-darwin/release/forge_main"
            }
        ]
    });

    let build_job = workflow.jobs.clone().unwrap().get("build").unwrap().clone();
    let main_cond =
        Expression::new("github.event_name == 'push' && github.ref == 'refs/heads/main'");

    // Add release build job
    workflow = workflow.add_job(
        "build-release",
        Job::new("build-release")
            .add_needs(build_job.clone())
            .cond(main_cond.clone())
            .strategy(Strategy { fail_fast: None, max_parallel: None, matrix: Some(matrix) })
            .runs_on("${{ matrix.os }}")
            .add_step(Step::uses("actions", "checkout", "v4"))
            // Install Rust with cross-compilation target
            .add_step(
                Step::uses("dtolnay", "rust-toolchain", "stable")
                    .with(("targets", "${{ matrix.target }}")),
            )
            // Build release binary
            .add_step(
                Step::uses("ClementTsang", "cargo-action", "v0.0.3")
                    .add_with(("command", "build --release"))
                    .add_with(("args", "--target ${{ matrix.target }}")),
            )
            // Upload artifact for release
            .add_step(
                Step::uses("actions", "upload-artifact", "v4")
                    .add_with(("name", "${{ matrix.binary_name }}"))
                    .add_with(("path", "${{ matrix.binary_path }}"))
                    .add_with(("if-no-files-found", "error")),
            ),
    );
    // Store reference to build-release job
    let build_release_job = workflow
        .jobs
        .clone()
        .unwrap()
        .get("build-release")
        .unwrap()
        .clone();

    // Add draft release job
    workflow = workflow.add_job(
        "draft_release",
        Job::new("draft_release")
            .runs_on("ubuntu-latest")
            .cond(main_cond.clone())
            .permissions(
                Permissions::default()
                    .contents(Level::Write)
                    .pull_requests(Level::Write),
            )
            .add_step(Step::uses("actions", "checkout", "v4"))
            .add_step(
                Step::uses("release-drafter", "release-drafter", "v6")
                    .id("create_release")
                    .env(("GITHUB_TOKEN", "${{ secrets.GITHUB_TOKEN }}"))
                    .with(("config-name", "release-drafter.yml")),
            )
            .add_step(
                Step::run("echo \"create_release_id=${{ steps.create_release.outputs.id }}\" >> $GITHUB_OUTPUT && echo \"create_release_name=${{ steps.create_release.outputs.tag_name }}\" >> $GITHUB_OUTPUT")
                    .id("set_output"),
            )
            .outputs(indexmap! {
                "create_release_name".to_string() => "${{ steps.set_output.outputs.create_release_name }}".to_string(),
                "create_release_id".to_string() => "${{ steps.set_output.outputs.create_release_id }}".to_string()
            })
    );

    // Store reference to draft_release job
    let draft_release_job = workflow
        .jobs
        .clone()
        .unwrap()
        .get("draft_release")
        .unwrap()
        .clone();

    // Store reference to create_release job before we add it
    let create_release_job = Job::new("create_release")
        .add_needs(build_release_job.clone())
        .add_needs(draft_release_job.clone())
        .cond(main_cond.clone())
        .permissions(
            Permissions::default()
                .contents(Level::Write)
                .pull_requests(Level::Write),
        )
        .runs_on("ubuntu-latest")
        .env(("GITHUB_TOKEN", "${{ secrets.GITHUB_TOKEN }}"))
        .add_step(Step::uses("actions", "checkout", "v4"))
        // Download all artifacts
        .add_step(
            Step::uses("actions", "download-artifact", "v4")
                .add_with(("pattern", "forge-*"))
                .add_with(("path", "artifacts")),
        )
        // Create directory for renamed artifacts
        .add_step(Step::run("mkdir -p renamed_artifacts"))
        // Rename and move artifacts with target names
        .add_step(Step::run(
            r#"for dir in artifacts/forge-*; do
                for file in "$dir"/*; do
                    if [ -f "$file" ]; then
                        filename=$(basename "$file")
                        dirname=$(basename "$dir")
                        target=${dirname#forge-}
                        cp "$file" "renamed_artifacts/forge-$target"
                    fi
                done
            done"#,
        ))
        // List artifacts
        .add_step(Step::run("ls -la renamed_artifacts/"))
        // Upload all renamed artifacts
        .add_step(
            Step::uses("xresloader", "upload-to-github-release", "v1")
                .add_with((
                    "release_id",
                    "${{ needs.draft_release.outputs.create_release_id }}",
                ))
                .add_with(("file", "renamed_artifacts/*"))
                .add_with(("overwrite", "true")),
        );

    // Add create_release job
    workflow = workflow.add_job("create_release", create_release_job.clone());

    // Add semantic release job to publish the release
    workflow = workflow.add_job(
        "semantic_release",
        Job::new("semantic_release")
            .add_needs(draft_release_job.clone())
            .add_needs(create_release_job.clone())
            .cond(Expression::new("(startsWith(github.event.head_commit.message, 'feat') || startsWith(github.event.head_commit.message, 'fix')) && (github.event_name == 'push' && github.ref == 'refs/heads/main')"))
            .permissions(
                Permissions::default()
                    .contents(Level::Write)
                    .pull_requests(Level::Write),
            )
            .runs_on("ubuntu-latest")
            .env(("GITHUB_TOKEN", "${{ secrets.GITHUB_TOKEN }}"))
            .env(("APP_VERSION", "${{ needs.draft_release.outputs.create_release_name }}"))
            .add_step(
                Step::uses("test-room-7", "action-publish-release-drafts", "v0")
                    .env(("GITHUB_TOKEN", "${{ secrets.GITHUB_TOKEN }}"))
                    .add_with(("github-token", "${{ secrets.GITHUB_TOKEN }}"))
                    .add_with(("tag-name", "${{ needs.draft_release.outputs.create_release_name }}")),
            ),
    );

    workflow.generate().unwrap();
}
#[test]
fn test_release_drafter() {
    // Generate Release Drafter workflow
    let mut release_drafter = Workflow::default()
        .on(Event {
            push: Some(Push { branches: vec!["main".to_string()], ..Push::default() }),
            pull_request_target: Some(PullRequestTarget {
                types: vec![
                    PullRequestType::Opened,
                    PullRequestType::Reopened,
                    PullRequestType::Synchronize,
                ],
                branches: vec!["main".to_string()],
                ..PullRequestTarget::default()
            }),
            ..Event::default()
        })
        .permissions(
            Permissions::default()
                .contents(Level::Write)
                .pull_requests(Level::Write),
        );

    release_drafter = release_drafter.add_job(
        "update_release_draft",
        Job::new("update_release_draft")
            .runs_on("ubuntu-latest")
            .add_step(
                Step::uses("release-drafter", "release-drafter", "v6")
                    .env(("GITHUB_TOKEN", "${{ secrets.GITHUB_TOKEN }}"))
                    .add_with(("config-name", "release-drafter.yml")),
            ),
    );

    release_drafter = release_drafter.name("Release Drafter");
    Generate::new(release_drafter)
        .name("release-drafter.yml")
        .generate()
        .unwrap();
}
