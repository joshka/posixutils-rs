use crate::builtin::{skip_option_terminator, BuiltinResult, SpecialBuiltinUtility};
use crate::shell::opened_files::OpenedFiles;
use crate::shell::Shell;

pub struct Eval;

impl SpecialBuiltinUtility for Eval {
    fn exec(
        &self,
        args: &[String],
        shell: &mut Shell,
        opened_files: &mut OpenedFiles,
    ) -> BuiltinResult {
        let args = skip_option_terminator(args);
        let program = args.join(" ");

        std::mem::swap(&mut shell.opened_files, opened_files);
        let execution_result = shell.execute_program(&program);
        std::mem::swap(&mut shell.opened_files, opened_files);

        execution_result
            .map_err(|err| format!("eval: parsing error({}): {}", err.lineno, err.message).into())
    }
}
