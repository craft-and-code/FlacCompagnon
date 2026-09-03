// Copying text to the system clipboard from the webview.
//
// Isolated here rather than inlined at the one call site because the fallback
// below is the kind of thing that gets copy-pasted the moment a second button
// needs it, and then only one of the two gets fixed.

/// Copy `text`, returning whether it worked.
///
/// Two paths, and the second is not decoration. `navigator.clipboard` needs a
/// secure context, which a Tauri webview normally is — but it is served over a
/// custom protocol whose treatment differs across the platform webviews this
/// app ships on, and a rejected promise there would leave a button that
/// silently does nothing. `document.execCommand("copy")` is deprecated and
/// still works everywhere, so it stands in when the modern API is missing or
/// refuses.
///
/// The fallback reaches into the DOM directly, which the rest of the frontend
/// does not do — but this is not state React could have rendered: the browser
/// only copies from a real selection in a real focused element, and the
/// element exists for the duration of one synchronous call.
export async function copyText(text: string): Promise<boolean> {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return true;
    }
  } catch {
    // Fall through — a rejection here is exactly the case the fallback is for.
  }
  return legacyCopy(text);
}

function legacyCopy(text: string): boolean {
  const area = document.createElement("textarea");
  area.value = text;
  // Off-screen rather than `display: none`: a hidden element cannot be
  // selected, and an unselected one cannot be copied from.
  area.style.position = "fixed";
  area.style.top = "-1000px";
  area.style.opacity = "0";
  // Keeps a screen reader from announcing the scratch element.
  area.setAttribute("aria-hidden", "true");
  document.body.appendChild(area);
  try {
    area.select();
    return document.execCommand("copy");
  } catch {
    return false;
  } finally {
    area.remove();
  }
}
