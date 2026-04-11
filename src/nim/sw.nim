;; Atrium Service Worker (Nim -> WASM)
;; 
;; Compile with: nim c --target:wasm32 sw.nim

import wasm

proc onFetch(event: ptr cstring): void {.exportc.} =
  # Handle fetch events
  discard

proc onInstall(event: ptr cstring): void {.exportc.} =
  # Handle install events
  discard

proc onActivate(event: ptr cstring): void {.exportc.} =
  # Handle activate events
  discard

# Entry point
when isMainModule:
  echo "Service Worker initialized"
