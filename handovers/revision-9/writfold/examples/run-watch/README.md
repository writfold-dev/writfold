# Run Watch design fixture

All records are synthetic. [The five original screens](approved-screens.pdf) are the approved **design baseline**. [Plan 27](../../plan/27-run-watch-and-session-handoff.md) owns the CLI design and acceptance cases. [The driver candidate](session-driver/SKILL.md) is the proposed agent-facing entry. Its reference is package-local. During import, pin both file digests and qualify the host projection against plan/27. This is not a host installation or an admitted procedure.

The planned agent command is `writfold run start --request start.json --json` from this directory on a configured WSL host. [The returned record](start-result.json) illustrates a committed start. [The later snapshot](show-34.json) corresponds to Gates and Pipeline in the mock booklet. RUN-017 is an illustrative ID. At #52 the view is two events behind a sampled head of #54. At #55 an independent lookup has confirmed the original effect. Replaying the examples requires no live provider.

Demo conversation for the mock's PR-opening boundary:

> Tarek: Track the pagination change across API, SDK and CLI through opening the reviewed PRs.
>
> Agent, after the start receipt: Run `demo/RUN-017`. Watch from a second WSL terminal with `writfold run watch demo/RUN-017`. I’ll stop at opened, reviewed PRs.
>
> Tarek: What is blocking the SDK?
>
> Agent, at #34: C7 has two matching spans at sdk@d02. CP-12 is waiting for a binding witness. Inspect that cut with `writfold run show demo/RUN-017 --at 34`.
