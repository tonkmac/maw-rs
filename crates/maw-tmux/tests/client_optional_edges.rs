use maw_tmux::{
    GroupedSessionOptions, NewSessionOptions, SplitWindowOptions, TmuxClient, TmuxError, TmuxRunner,
};
use std::{cell::RefCell, rc::Rc};

type CapturedCalls = Rc<RefCell<Vec<(String, Vec<String>)>>>;

#[derive(Clone, Default)]
struct RecordingRunner {
    calls: CapturedCalls,
    responses: Rc<RefCell<Vec<Result<String, TmuxError>>>>,
}

impl RecordingRunner {
    fn with_responses(responses: Vec<Result<&str, TmuxError>>) -> Self {
        Self {
            calls: Rc::default(),
            responses: Rc::new(RefCell::new(
                responses
                    .into_iter()
                    .map(|response| response.map(str::to_owned))
                    .collect(),
            )),
        }
    }

    fn calls(&self) -> Vec<(String, Vec<String>)> {
        self.calls.borrow().clone()
    }
}

impl TmuxRunner for RecordingRunner {
    fn run(&mut self, subcommand: &str, args: &[String]) -> Result<String, TmuxError> {
        self.calls
            .borrow_mut()
            .push((subcommand.to_owned(), args.to_vec()));
        self.responses.borrow_mut().remove(0)
    }
}

#[test]
fn tmux_client_optional_session_args_can_be_omitted() {
    let runner = RecordingRunner::with_responses(vec![Ok("%1\n"), Ok("")]);
    let calls = runner.clone();
    let mut client = TmuxClient::new(runner);
    let out = client
        .new_session(
            "plain",
            &NewSessionOptions {
                detached: false,
                ..NewSessionOptions::default()
            },
        )
        .expect("new session");

    assert_eq!(out, "%1\n");
    assert_eq!(
        calls.calls()[0],
        (
            "new-session".to_owned(),
            vec!["-s".to_owned(), "plain".to_owned()]
        )
    );
}

#[test]
fn tmux_client_optional_grouped_and_split_args_can_be_omitted() {
    let runner = RecordingRunner::with_responses(vec![Ok(""), Ok("%2\n")]);
    let calls = runner.clone();
    let mut client = TmuxClient::new(runner);

    client
        .new_grouped_session("parent", "child", &GroupedSessionOptions::default())
        .expect("grouped session");
    let split = client
        .split_window(None, &SplitWindowOptions::default())
        .expect("split window");

    assert_eq!(split, "%2\n");
    assert_eq!(
        calls.calls(),
        vec![
            (
                "new-session".to_owned(),
                vec![
                    "-d".to_owned(),
                    "-t".to_owned(),
                    "parent".to_owned(),
                    "-s".to_owned(),
                    "child".to_owned(),
                ],
            ),
            ("split-window".to_owned(), Vec::new()),
        ]
    );
}
