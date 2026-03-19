# The following variables are available:
# • ZED_ROW:                 current line row
# • ZED_COLUMN:              current line column
# • ZED_SELECTED_TEXT:       currently selected text
# • ZED_FILENAME:            filename of the currently opened file (e.g. main.rs)
# • ZED_CUSTOM_RUST_PACKAGE: (Rust-specific) name of the parent package of $ZED_FILE source file.
# • ZED_STEM:                stem (filename without extension) of the currently opened file (e.g. main)
# • ZED_RELATIVE_FILE:       path of the currently opened file, relative to ZED_WORKTREE_ROOT (e.g. src/main.rs)
# • ZED_RELATIVE_DIR:        path of the currently opened file's directory, relative to ZED_WORKTREE_ROOT (e.g. src)
# • ZED_WORKTREE_ROOT:       absolute path to the root of the current worktree. (e.g. /Users/my-user/path/to/project)
# • ZED_FILE:                absolute path of the currently opened file (e.g. /Users/my-user/path/to/project/src/main.rs)
# • ZED_DIRNAME:             absolute path of the currently opened file with file name stripped (e.g. /Users/my-user/path/to/project/src)
# • ZED_SYMBOL:              currently selected symbol; should match the last symbol shown in a symbol breadcrumb (e.g. mod tests > fn test_task_contexts)


export def run-example [] {
	# this lets rustanalyzer display all the warnings, but on compile warnings are supressed so thats cool
	$env.RUSTFLAGS = "-A unused_variables"
	match ($env.ZED_RELATIVE_FILE | path split) {
		["examples", $name, ..] => { ^cargo run --example ($name | str replace ".rs" "") }
		_ => { error make -u { msg: $"Editor File: ((ansi i) + $env.ZED_RELATIVE_FILE + (ansi rst))\nNot inside ((ansi yi) + "`examples`" + (ansi rst)) directory." } }
	}
}
