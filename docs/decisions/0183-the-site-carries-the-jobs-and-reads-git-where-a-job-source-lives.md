# 0183. The site carries the jobs, and reads git where a job source lives

Date: 2026-11-23 · Status: accepted · Amends 0177 and 0115

**Trigger:** read before changing what `--site` reads of the orchestrator's
jobs, what the page does when a source was not read, how a link to a job file
is made, or which folder git is asked about besides the project's.

## Context

The static site (0177) left the jobs out, with the trap written down: the Jobs
tab's answers are not pure functions of the manifest. A step's `--select` is
resolved by the selection engine, which `web/site.js` cannot be, and the files
are in another repository that the pipeline clones beside the dbt one. A reader
of the documentation asks which DAGs and jobs run a model, and the job file is
the proof. 0177 says no job file is read, and 0115 that the command line reads
`.dbt-edith/jobs.yml` and no settings.

## Decision

- **The jobs are read once, when the site is written, from `.dbt-edith/jobs.yml`
  alone**, as the command line reads them (0115): the file's own limits hold
  (two folders above the project at most, nothing hidden, no `profiles.yml`,
  where a link really leads), and the settings are never read. No file, no
  jobs, no tab. A file that lists no source is no file.
- **Everything is computed in Rust.** The page asks `/api/jobs` and
  `/api/jobs/node`, and `web/site.js` answers them from `data/jobs.js`, one
  script loaded whole when the Jobs tab or a card's Jobs line first asks. The
  file holds `jobs::summary`'s answer and the steps, laid out so that nothing
  grows with the product of nodes and steps. Each job and each step is written
  once, in the order of the files, a step as `StepView` writes it for no node in
  particular. A step that runs nodes says which as runs of node numbers
  (`[gap, length, ...]`, so a step that selects everything is one run), and why,
  by node, as the terms of its `--select`, each with the nodes it reaches on its
  own (nothing more when that is what the step runs) or as the one line of a step
  that names a selector or none. The steps that could not be resolved and might
  run a node are said once for each kind of node. What every node's answer says
  alike, the sources read, those that were not and the files refused, is said
  once. `web/site.js` finds a node's steps by looking it up in each step's runs,
  groups them by job as the server does, and puts the server's shape back; the
  golden file holds it to `jobs::for_node` for every node of an invented graph,
  and a Rust test does the same for another. A term is resolved once however
  many steps name it.
- **What the answers do not hold:** where a source led on this machine
  (`resolved`) is dropped, a source is as `jobs.yml` wrote it, and so are the
  settings form's keys and the version of the file. The commands in the DAG files
  are the files' own content and are carried as they are. No time is written: the
  Jobs tab says the jobs were read when the site was written, and the header
  already says when the manifest was. The same inputs give the same bytes.
- **A source that was not read is said, never as no job.** A folder that is not
  there, a walk that stopped at its bound and a `jobs.yml` that does not read give
  a site written all the same, exit 0, with `warning: jobs not read: …` on stderr.
  The page says "Jobs not read: …", with the server's own reason, on the card
  and in the tab, in place of "No job read runs it" and of "No step of the jobs
  read acts on it", which would claim what nobody opened a file to know.
- **Links to the job files are the jobs repository's, at its commit.** For each
  source that was read, git is asked, at the folder the source really leads to and
  through the gate (0121, hooks off), for the top of the working tree, `HEAD`,
  `remote.origin.url` and where the folder sits (`--show-prefix`): four reads, and
  nothing else, with the top of the project's own tree, asked once for the rule
  below. The pattern is the host's and the origin's rule is 0177's, as for
  the project's own files (0180); a line is `#L<n>` on GitHub, GitLab and a host
  named by flag, `#lines-<n>` on Bitbucket, and there is none for Azure DevOps,
  which keeps its commit. `--jobs-commit` and `--jobs-repo-url` are checked like
  `--commit` and `--repo-url` and apply to every source; a pipeline that cannot
  read git passes them. The Jobs tab's header names each source's short commit as
  a link.
- **The repository found is the sources' own, or it is none.** git walks up from a
  folder with no ceiling, so a jobs folder that is not a repository would be placed
  in whatever holds its parents, a dotfiles repository at the home folder for one,
  and its commit and origin would be linked as the jobs'. The top found is taken
  only when it is the dbt project's own repository (jobs kept in the project) or
  does not hold the project at all, and never when it is at the home folder or above
  it (`HOME`, `USERPROFILE` on Windows; not asked when it is unset, as `resolve_real`'s
  own check is not). Otherwise git is not asked, no file is linked, since the
  folder's place in the repository would be a guess, and stderr says why; the
  commit and its link are still the flags', when both are given, and so they are
  with no repository at all.
- **The project's own git reads follow the same rule about the home folder.** A
  repository at the home folder or above it is no project's own: the commit,
  origin and prefix of PR 1 and 2 (0177, 0180) are not read from it, and `--commit`
  and `--repo-url` are what remain. This amends 0177 and 0180 on that point only.
- **The rest of the page stays as 0177 left it.** Sources…, the `dbt ls` button,
  the Coverage button, the file picker and the routes that save or browse are not
  in a site; Reports, with its coverage, job order and builds by job, is not in
  it either. A file button is a link where there is a base and keeps copying
  `file:line` where there is none.

## Rejected

- **An entry per node, or per node and step.** The first form of this file
  wrote each node's groups, its steps and the terms that reached it, and the
  `maybe` steps again for every node: on an invented project with broad
  selections, its time, its memory and its size grew with the nodes times the
  steps.
  What a step runs is a set of nodes, and a set of nodes is runs; what might run
  a node depends on its kind, not on the node.
- **Resolving the selections in the browser.** The selection engine is large and
  the server's answer is the one a person compares with `dbt ls` (0033).
- **A time in the file.** "Read at" would make two runs of the same inputs two
  sets of bytes, which 0177 promises they are not.
- **Taking whatever repository git finds.** The link would name the home
  folder's commit as the jobs'.
- **A pair of flags per source.** A pipeline with several repositories of jobs
  has none yet; the flags apply to all and a second pair is for whoever asks.
- **Failing the run for a missing repository.** The pipeline that clones the
  jobs repository can fail to; the site is still the manifest's picture, and a
  page that says what it did not read is better than none.
