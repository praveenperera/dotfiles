# Supervise resource loans

Use `homebased resource --help` for the installed commands and
`homebased --json resource schema` for the accepted JSON shapes.

## Required checks

Use the resource CLI. Do not use ordinary `task submit` for GPU work that
belongs to a registered resource. Do not bypass an active loan or FIFO request
queue. Do not monitor a loan by polling with a model or in a command loop.
Queued GPU commands must run native foreground executables. Scripts,
interpreters, shell wrappers, container clients, and detach tools are refused.

Check the exact actions for the assigned supervisor machine and thread:

- when resource work starts
- after compaction or resume
- before launching any independent background task
- after a notice or an unknown action result, when needed to resolve state

Use `homebased --json resource pending --machine <machine-uuid> --thread
<thread-uuid>`. Read `unavailable_authorities` before interpreting `actions`.
If any authority is unavailable, the result is incomplete. Do not treat it as
no pending action and do not start independent GPU work. If all authorities
answered and `actions` is empty, there is no pending supervisor action for that
exact address; still check `resource show` for an active loan and
`resource requests` for FIFO work.

A delivered notice does not complete an action. Read the action phase from a
fresh authority-backed pending result. Keep the saved pending JSON unchanged for
an unknown-outcome retry. Keep the action, request, task, and operation IDs
unchanged as the runbook requires. If the authority returns a definite
rejection, do not retry with changed content under the same ID.

## Do not bypass release or return ownership

The resource authority owns release proof. A checkpoint by itself does not
prove that the task stopped or released its ownership lock. There is no public
`resource released` command. `ResourceActor` reconciles authority-built proof.

A failed trainer, or a trainer cancelled without the action's saved stop, gives
the `ended_without_result` return context. The authority gives it only after
it proves that the exact saved lock is free. Do not resume that run. Choose
`after_ended_run` or `--no-resume`. A trainer with no trainer-attempt
association stays reserved with `TrainerAssociationMissing`. Do not work around
it with another launch. After the trainer ends or is lost, an operator may use
`resource operator-release --spec <saved-document>` on the GPU authority machine
only after inspecting that machine and confirming that no trainer GPU work
remains. This is a human attestation, not automatic exit or lock proof. Use the
exact task, resource revision, and `state_binding` from `resource show`:

- `no_loan` or `awaiting_release` for the registered trainer.
- `first_background_launch` with `background_launch.request_id` and
  `background_launch.task_id` when `background_launch.status` is
  `release_unproven`. This first launch ended before registration. The
  dashboard shows `Operator release required`, even with no queued request.
- When `loan` is `restoring`, use the exact `return_execution_mode` value from
  `resource show`: `direct_segment_trainer` selects `restoring_return`, and
  `native_foreground` selects `restoring_foreground_return`. Use the Restoring
  loan and action UUIDs and `resume_task_id` for the binding. If the field is
  absent or unknown, do not guess from the decision, task status, command name,
  or command text, and do not submit either binding.
- A native foreground return task keeps the Restoring loan until a successful
  end with a confirmed process-group exit. An operator attestation is not exit
  proof, and the task keeps its saved state.

See the [operator procedure](../../../../docs/resource-loans.md#resolve-an-unproven-ended-trainer).
Keep the document and operation ID unchanged after an unknown outcome, and
retry on the same authority. The command refuses a queued or running task.

Use `resource release-watch` only for a remote supervisor action. A co-located
authority starts its watcher. Do not start another watcher. `resource return`
and `resource resolve` work for remote and co-located supervisors. Keep the
same action, request, and task IDs when an outcome is unknown.

A native foreground return task keeps the `restoring` reservation until it
exits with code 0 and a confirmed process-group exit. It never becomes the
registered training task, so do not expect a release action for it. Use
`resource resolve` for a failed foreground end.

Do not use the current live training job to test, verify, or demonstrate this
workflow. Use a controlled registered task.
