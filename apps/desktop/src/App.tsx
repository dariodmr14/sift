import { useRef, useState } from "react";
import { commands } from "./bindings";
import { useLauncher } from "./useLauncher";

function App() {
  const [query, setQuery] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  useLauncher(() => {
    setQuery("");
    inputRef.current?.focus();
  });

  function onKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    if (e.key === "Escape") {
      e.preventDefault();
      void commands.hideLauncher();
    }
  }

  return (
    <main className="flex h-full flex-col bg-(--surface)">
      <input
        ref={inputRef}
        autoFocus
        value={query}
        onChange={(e) => setQuery(e.currentTarget.value)}
        onKeyDown={onKeyDown}
        placeholder="Buscar en Sift…"
        spellCheck={false}
        aria-label="Buscar"
        className="w-full border-b border-(--border) bg-transparent px-5 py-4 text-xl text-(--text) outline-none placeholder:text-(--text-muted)"
      />
    </main>
  );
}

export default App;
