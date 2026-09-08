# Work Orders

Work Orders are the only normal authorization mechanism for implementation agents.

## Required fields

Every Work Order must define:

- objective;
- exact change surface;
- dependencies;
- forbidden surfaces;
- architecture references;
- behavioral acceptance criteria;
- verification commands/evidence;
- completion report requirements.

## Rules

A Work Order may implement frozen architecture only. If implementation reveals that frozen architecture must change, stop and produce an Architecture Change Request instead of silently redesigning the system.

One bounded Work Order per implementation branch/PR unless explicitly composed by the Work Order.

A Work Order is not complete until its acceptance evidence is tied to the exact resulting commit SHA.
