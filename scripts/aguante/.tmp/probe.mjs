import { connectBar, invoke } from "../comun.mjs";
const bar = await connectBar();
const out = {};
out.status = await invoke(bar, "index_status");
out.roots = await invoke(bar, "list_roots");
if (out.roots.length) out.unread = await invoke(bar, "unread_files", { rootId: out.roots[0].id });
out.st = await invoke(bar, "get_status");
console.log(JSON.stringify(out, null, 1));
process.exit(0);
