from pathlib import Path
import tomllib

def verify():
    print("[verify] Checking contract and explorer implementation...")
    root = Path(__file__).resolve().parent.parent
    src = (root / "src" / "main.rs").read_text(encoding="utf-8")
    session = (root / "src" / "session.rs").read_text(encoding="utf-8")
    explorer = (root / "src" / "explorer.rs").read_text(encoding="utf-8")

    # 1. Print stylesheet check
    assert "@page {{\n  margin: 12mm;\n}}" in src, "Missing print margin"
    assert "@media print {{" in src and "#app {{ max-width: none; padding: 0; }}" in src, "Missing print rules"

    # 2. Desktop tabs & existing contract
    assert 'id="tabbar"' in src and "window.__setTabs" in src, "Missing tab bar"
    assert "session.json" in src and "PersistedSession" in session, "Missing session persistence"
    assert '"Close Tab"' in src and "mdPreviewCloseTab:" in src, "Missing Close Tab"
    assert "window.__setMissing" in src and "data-locate-tab" in src, "Missing missing-tab handling"
    for marker in ("new-file", "AUTOSAVE_DEBOUNCE_MS = 700", "window.__mdPreviewResolveExternalChange", "UserEvent::Quit"):
        assert marker in src, f"Missing marker: {marker}"

    # 3. Explorer & Sidebar Contract
    assert 'id="sidebar"' in src, "Missing #sidebar DOM element"
    assert 'id="btn-sidebar"' in src, "Missing #btn-sidebar toolbar button"
    assert 'id="tree-view"' in src, "Missing #tree-view container"
    assert "window.__setExplorerTree" in src, "Missing window.__setExplorerTree JS function"
    assert "window.__setActiveExplorerPath" in src, "Missing window.__setActiveExplorerPath JS function"
    assert "window.__mdPreviewToggleSidebar" in src, "Missing window.__mdPreviewToggleSidebar JS function"
    assert "open-explorer-file:" in src, "Missing open-explorer-file IPC message handler"
    assert "open-explorer-pinned:" in src, "Missing pinned explorer-file IPC message handler"
    assert ".tab.preview .tab-name" in src, "Missing preview-tab styling"
    assert "open_preview" in session and "tab.preview" in session, "Missing preview-tab session state"
    assert "requestTabAction('pin'" in src, "Missing tab double-click pinning"
    assert "(e.key === 'b' || e.key === 'B')" in src, "Missing Cmd/Ctrl+B shortcut"
    assert "scan_directory" in explorer, "Missing scan_directory in explorer.rs"
    assert "find_default_document" in explorer, "Missing find_default_document in explorer.rs"
    assert "WorkspaceTree" in explorer, "Missing WorkspaceTree struct"
    assert "FileEntry" in explorer, "Missing FileEntry struct"
    assert "workspace_root" in session, "Missing workspace_root in DocumentSession"

    # 4. Product version consistency
    version = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
    readme = (root / "README.md").read_text(encoding="utf-8")
    assert version in (root / "docs" / "index.html").read_text(encoding="utf-8"), "Version mismatch in docs/index.html"
    for marker in ("Workspace explorer", "Preview tabs", "fork release checklist"):
        assert marker in readme, f"README.md is missing explorer documentation: {marker}"

    print("[verify] ALL VERIFICATION CONTRACT CHECKS PASSED SUCCESSFULLY!")

if __name__ == "__main__":
    verify()
