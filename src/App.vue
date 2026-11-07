<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { ask, open, save } from "@tauri-apps/plugin-dialog";
import iconUrl from "../src-tauri/icons/icon.png";
import { version } from "../package.json";
import thirdPartyNotices from "../THIRD-PARTY-NOTICES.md?raw";
import { type Child, type Handle, type Tag, UNKNOWN_ID, MIN_TAG_SIZE, clamp, cleanFolderName, decodeText, editBox, exportFolderName, exportJobs, findChild, forgetTags, isMaskedFor, jpegName, maskEllipse, parseNames } from "./logic";

type Photo = { id: string; name: string; file: File; url: string; tags: Tag[] };

const roster = ref<Child[]>([]);
const photos = ref<Photo[]>([]);
const selected = ref(0);
const unlocked = ref(false);
// vault : coffre-fort du système ; legacy : fichier illisible ; setup/unlock : repli par phrase de passe (Linux sans coffre-fort).
const lockMode = ref<"vault" | "legacy" | "setup" | "unlock">("vault");
const lockPassphrase = ref("");
const lockPassphraseConfirm = ref("");
const showLockPassphrase = ref(false);
const childName = ref("");
const childAllowed = ref(false);
const editingChild = ref<Child | null>(null);
const passphrase = ref("");
const maskMode = ref<"blur" | "fill">("blur");
const selectedChildId = ref<number | null>(null);
const notice = ref("");
const working = ref(false);
const exportProgress = ref<{ done: number; total: number } | null>(null);
const imageFrame = ref<HTMLElement>();
const aboutDialog = ref<HTMLDialogElement>();
const canvasBox = ref<HTMLElement>();
const photoImg = ref<HTMLImageElement>();
const photoInput = ref<HTMLInputElement>();
const rosterInput = ref<HTMLInputElement>();
const stageSection = ref<HTMLElement>();
// Erreur de saisie affichée sous le champ concerné (RGAA 11.10) : id du champ + message.
const fieldError = ref<{ field: string; text: string } | null>(null);
const errorFor = (field: string) => fieldError.value?.field === field ? fieldError.value.text : "";
const zoom = ref(1);
const baseWidth = ref(0);
const MAX_ZOOM = 6;
const drawing = ref<{ x: number; y: number } | null>(null);
const draft = ref<{ x: number; y: number; width: number; height: number } | null>(null);
const handles: Handle[] = ["nw", "ne", "sw", "se"];
const editing = ref<{ tag: Tag; handle: Handle; start: { x: number; y: number }; origin: { x: number; y: number; width: number; height: number } } | null>(null);

const current = computed(() => photos.value[selected.value]);
const childById = (id: number) => findChild(roster.value, id);
// Un cadre dont l'élève est introuvable est traité comme non autorisé.
const isMasked = (tag: Tag) => !childById(tag.childId)?.allowsDistribution;
const displayTags = computed(() => current.value?.tags ?? []);
const selectedChild = computed(() => selectedChildId.value === null ? undefined : childById(selectedChildId.value));
const photoCount = (id: number) => photos.value.filter((photo) => photo.tags.some((tag) => tag.childId === id)).length;
const taggedPhotos = computed(() => photos.value.filter((photo) => photo.tags.length).length);
const tagSignature = computed(() => JSON.stringify(photos.value.map((photo) => [photo.id, photo.tags])));
const exportedSignature = ref("");
const hasUnexportedWork = computed(() => taggedPhotos.value > 0 && tagSignature.value !== exportedSignature.value);
const imgStyle = computed(() => zoom.value > 1 ? { width: `${baseWidth.value * zoom.value}px`, maxWidth: "none", maxHeight: "none" } : undefined);
watch(selected, () => { zoom.value = 1; });

function toggleChild(id: number) { selectedChildId.value = selectedChildId.value === id ? null : id; }
function onKey(event: KeyboardEvent) {
  const target = event.target as HTMLElement;
  if (["INPUT", "SELECT", "TEXTAREA"].includes(target.tagName) || aboutDialog.value?.open) return;
  // Ctrl/⌘ + - / = sont réservés au zoom du texte de l'application (accessibilité) ; Ctrl/⌘ + 0 le remet à 100 %.
  if ((event.ctrlKey || event.metaKey) && event.key === "0") { getCurrentWebview().setZoom(1).catch(() => {}); return; }
  if (event.ctrlKey || event.metaKey || event.altKey) return;
  if (event.key === "ArrowLeft" && selected.value > 0) selected.value--;
  if (event.key === "ArrowRight" && selected.value < photos.value.length - 1) selected.value++;
  // Raccourcis d'une seule touche (RGAA 12.10) : actifs seulement quand le focus est dans la zone photo.
  if (!stageSection.value?.contains(target)) return;
  if (event.key === "+" || event.key === "=") setZoom(zoom.value * 1.5);
  if (event.key === "-") setZoom(zoom.value / 1.5);
  if (event.key === "0") zoom.value = 1;
}
let unlistenClose: (() => void) | undefined;
onMounted(async () => {
  window.addEventListener("keydown", onKey);
  unlistenClose = await getCurrentWindow().onCloseRequested(async (event) => {
    if (!hasUnexportedWork.value) return;
    const quit = await ask("Des photos identifiées n’ont pas été exportées depuis la dernière modification. Si vous quittez, ce travail d’identification sera perdu (les photos d’origine ne sont pas touchées).",
      { title: "Quitter sans exporter ?", kind: "warning", okLabel: "Quitter", cancelLabel: "Rester" });
    if (!quit) event.preventDefault();
  });
});

// Vider puis réécrire : un message identique au précédent est ainsi annoncé à nouveau.
function message(value: string) { notice.value = ""; nextTick(() => { notice.value = value; }); }

// Place le focus (après rendu) sur l'élément désigné par un sélecteur CSS (RGAA 12.8).
async function focusOn(selector: string) {
  await nextTick();
  document.querySelector<HTMLElement>(selector)?.focus();
}
// Erreur liée à un champ : message sous le champ, aria-invalid, focus sur le champ.
function showFieldError(field: string, text: string) {
  fieldError.value = { field, text };
  focusOn(`#${field}`);
}
function clearFieldError(field: string) { if (fieldError.value?.field === field) fieldError.value = null; }

async function unlockedWith(action: () => Promise<unknown>, success: string) {
  working.value = true;
  try {
    await action();
    roster.value = await invoke<Child[]>("list_children");
    unlocked.value = true;
    lockPassphrase.value = lockPassphraseConfirm.value = "";
    fieldError.value = null;
    message(success);
    focusOn("#eleves-titre");
  } catch (error) {
    if (error === "LEGACY_ROSTER") { lockMode.value = "legacy"; message("La classe enregistrée sur cet ordinateur ne peut pas être ouverte."); }
    else if (error === "PASSPHRASE_SETUP") { lockMode.value = "setup"; notice.value = ""; }
    else if (error === "PASSPHRASE_REQUIRED") { lockMode.value = "unlock"; notice.value = ""; }
    else if (lockMode.value === "unlock" || lockMode.value === "setup") showFieldError("lock-passphrase", errorText(error));
    else message(errorText(error));
  } finally { working.value = false; }
}

const openRoster = () => unlockedWith(() => invoke("open_roster"), "La classe est prête.");

async function submitPassphrase() {
  const creating = lockMode.value === "setup";
  if (lockPassphrase.value.length < 8) { showFieldError("lock-passphrase", "La phrase de passe doit contenir au moins 8 caractères."); return; }
  if (creating && lockPassphrase.value !== lockPassphraseConfirm.value) { showFieldError("lock-confirm", "Les deux saisies ne sont pas identiques. Retapez la même phrase de passe dans les deux champs."); return; }
  await unlockedWith(() => invoke("open_roster_with_passphrase", { passphrase: lockPassphrase.value, create: creating }),
    creating ? "Classe créée. Retenez bien votre phrase de passe : elle vous sera demandée à chaque ouverture." : "La classe est prête.");
}

async function startNewClass() {
  working.value = true;
  try { await invoke("archive_legacy_roster"); } catch (error) { message(errorText(error)); return; } finally { working.value = false; }
  await unlockedWith(() => invoke("open_roster"), "Nouvelle classe prête. L’ancien fichier a été archivé, sans être supprimé.");
}

async function forgotPassphrase() {
  const confirmed = await ask("Sans la phrase de passe, la liste actuelle des élèves ne peut plus être lue, par personne. Vous pouvez commencer une nouvelle classe : l’ancienne liste sera mise de côté (archivée, pas supprimée), et vous choisirez une nouvelle phrase de passe.",
    { title: "Phrase de passe oubliée", kind: "warning", okLabel: "Commencer une nouvelle classe", cancelLabel: "Annuler" });
  if (confirmed) await startNewClass();
}

async function reloadRoster() { roster.value = await invoke<Child[]>("list_children"); }

async function addChild() {
  if (!childName.value.trim()) return;
  try {
    await invoke("save_child", { name: childName.value, allowsDistribution: childAllowed.value });
    await reloadRoster();
    message(`${childName.value.trim()} a été ajouté(e) à la classe.`);
    childName.value = "";
    childAllowed.value = false;
    fieldError.value = null;
  } catch (error) { showFieldError("child-name", errorText(error)); }
}

function startEditChild(child: Child) { editingChild.value = { ...child }; focusOn("#edit-name"); }
function stopEditChild(id: number) { editingChild.value = null; fieldError.value = null; focusOn(`[data-edit="${id}"]`); }
async function saveEditChild() {
  const edit = editingChild.value;
  if (!edit || !edit.name.trim()) return;
  try {
    await invoke("update_child", { id: edit.id, name: edit.name, allowsDistribution: edit.allowsDistribution });
    await reloadRoster();
    stopEditChild(edit.id);
  } catch (error) { showFieldError("edit-name", errorText(error)); }
}

function forgetChildren(ids: number[]) {
  forgetTags(photos.value, ids);
  if (selectedChildId.value !== null && ids.includes(selectedChildId.value)) selectedChildId.value = null;
}

async function deleteChild(child: Child) {
  const tagged = photoCount(child.id);
  const confirmed = await ask(`Supprimer ${child.name} de la classe ?${tagged ? ` Ses cadres dans les photos de cette session deviendront des « visages inconnus », toujours floutés.` : ""}`,
    { title: "Supprimer un élève", kind: "warning", okLabel: "Supprimer", cancelLabel: "Annuler" });
  if (!confirmed) return;
  try {
    await invoke("delete_child", { id: child.id });
    forgetChildren([child.id]);
    editingChild.value = null;
    await reloadRoster();
    message(`${child.name} a été retiré(e) de la classe.`);
    focusOn("#eleves-titre");
  } catch (error) { message(errorText(error)); }
}

async function clearRoster() {
  const confirmed = await ask("Effacer définitivement tous les élèves et leurs accords de diffusion ? À faire en fin d’année scolaire. Pensez à faire une sauvegarde avant si vous en avez besoin.",
    { title: "Nouvelle année scolaire", kind: "warning", okLabel: "Tout effacer", cancelLabel: "Annuler" });
  if (!confirmed) return;
  try {
    await invoke("clear_roster");
    forgetChildren(roster.value.map((child) => child.id));
    editingChild.value = null;
    await reloadRoster();
    message("La liste des élèves a été effacée.");
    focusOn("#eleves-titre");
  } catch (error) { message(errorText(error)); }
}

async function importRoster(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (!file) return;
  try {
    const names = parseNames(decodeText(await file.arrayBuffer()));
    if (!names.length) { message("Aucun nom trouvé dans ce fichier."); return; }
    const added = await invoke<number>("import_children", { names });
    await reloadRoster();
    const skipped = names.length - added;
    message(`${added} élève(s) ajouté(s), masqué(s) par défaut : cochez l’accord des familles qui ont autorisé la diffusion.${skipped ? ` ${skipped} nom(s) déjà présent(s) ignoré(s).` : ""}`);
  } catch (error) { message(`Import impossible : ${errorText(error)}`); }
}

const backupFilter = [{ name: "Sauvegarde de classe", extensions: ["sauvegarde"] }];
async function backupRoster() {
  if (passphrase.value.length < 8) { showFieldError("backup-passphrase", "Choisissez d’abord une phrase de passe d’au moins 8 caractères."); return; }
  const destination = await save({ title: "Enregistrer la sauvegarde de la classe", defaultPath: `classe-${new Date().toISOString().slice(0, 10)}.sauvegarde`, filters: backupFilter });
  if (!destination) return;
  try {
    await invoke("backup_roster", { destination, passphrase: passphrase.value });
    passphrase.value = "";
    fieldError.value = null;
    message("Sauvegarde chiffrée enregistrée. Conservez la phrase de passe : sans elle, la sauvegarde ne peut pas être rouverte.");
  } catch (error) { showFieldError("backup-passphrase", errorText(error)); }
}
async function restoreRoster() {
  if (passphrase.value.length < 8) { showFieldError("backup-passphrase", "Saisissez d’abord la phrase de passe de la sauvegarde (8 caractères minimum)."); return; }
  const source = await open({ title: "Choisir une sauvegarde de classe", multiple: false, directory: false, filters: backupFilter });
  if (!source || Array.isArray(source)) return;
  try {
    const count = await invoke<number>("restore_roster", { source, passphrase: passphrase.value });
    passphrase.value = "";
    await reloadRoster();
    fieldError.value = null;
    message(`${count} élève(s) restauré(s) depuis la sauvegarde.`);
  } catch (error) { showFieldError("backup-passphrase", errorText(error)); }
}

async function normaliseImage(file: File): Promise<File> {
  const isHeic = file.type === "image/heic" || /\.hei[cf]$/i.test(file.name);
  if (!isHeic) return file;
  // Le moteur système sait parfois lire le HEIC (WebKit sur macOS), ou le fichier est en réalité un JPEG/PNG :
  // pas de conversion nécessaire, l'export passe de toute façon par createImageBitmap.
  try {
    (await createImageBitmap(file)).close();
    return file;
  } catch { /* décodage natif indisponible : conversion locale ci-dessous */ }
  const { heicTo } = await import("heic-to/csp");
  let blob: Blob;
  try {
    blob = await heicTo({ blob: file, type: "image/jpeg", quality: 0.92 });
  } catch (error) {
    throw new Error(`${file.name} : conversion HEIC impossible (${errorText(error)})`);
  }
  return new File([blob], file.name.replace(/\.hei[cf]$/i, ".jpg"), { type: "image/jpeg" });
}

function errorText(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  if (error && typeof error === "object" && "message" in error) return String((error as { message: unknown }).message);
  return String(error);
}

async function importPhotos(event: Event) {
  const input = event.target as HTMLInputElement;
  const files = [...(input.files ?? [])];
  if (!files.length) return;
  working.value = true;
  let converted = 0;
  try {
    for (const source of files) {
      if (!source.type.startsWith("image/") && !/\.hei[cf]$/i.test(source.name)) continue;
      const file = await normaliseImage(source);
      if (file.name !== source.name) converted++;
      photos.value.push({ id: crypto.randomUUID(), name: file.name, file, url: URL.createObjectURL(file), tags: [] });
    }
    message(`${files.length} photo(s) ajoutée(s)${converted ? `, dont ${converted} HEIC convertie(s) localement en JPEG` : ""}.`);
  } catch (error) { message(`Import impossible : ${errorText(error)}`); } finally {
    working.value = false;
    input.value = "";
  }
}

function pointerPosition(event: PointerEvent) {
  const rect = imageFrame.value!.getBoundingClientRect();
  return { x: Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width)), y: Math.max(0, Math.min(1, (event.clientY - rect.top) / rect.height)) };
}
function beginTag(event: PointerEvent) {
  if (!current.value || selectedChildId.value === null) { message("Choisissez d'abord un élève, puis dessinez son cadre."); return; }
  drawing.value = pointerPosition(event);
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
}
function beginEdit(event: PointerEvent, tag: Tag, handle: Handle) {
  editing.value = { tag, handle, start: pointerPosition(event), origin: { x: tag.x, y: tag.y, width: tag.width, height: tag.height } };
  imageFrame.value!.setPointerCapture(event.pointerId);
}
function applyEdit(event: PointerEvent) {
  const { tag, handle, start, origin: o } = editing.value!;
  const end = pointerPosition(event);
  Object.assign(tag, editBox(o, handle, end.x - start.x, end.y - start.y));
}
function moveTag(event: PointerEvent) {
  if (editing.value) { applyEdit(event); return; }
  if (!drawing.value) return;
  const end = pointerPosition(event);
  draft.value = { x: Math.min(drawing.value.x, end.x), y: Math.min(drawing.value.y, end.y), width: Math.abs(end.x - drawing.value.x), height: Math.abs(end.y - drawing.value.y) };
}
function finishTag() {
  if (draft.value && current.value && selectedChildId.value !== null && draft.value.width > MIN_TAG_SIZE && draft.value.height > MIN_TAG_SIZE) {
    current.value.tags.push({ id: crypto.randomUUID(), childId: selectedChildId.value, ...draft.value });
  }
  drawing.value = null; draft.value = null; editing.value = null;
}
function removeTag(id: string) { if (current.value) current.value.tags = current.value.tags.filter((tag) => tag.id !== id); }
// Alternative clavier au dessin à la souris (RGAA) : Entrée ajoute un cadre au centre de la photo,
// flèches pour le déplacer, Maj + flèches pour le redimensionner, Suppr pour le retirer.
const KEY_STEP = 0.01;
async function addTagAtCenter() {
  if (!current.value) return;
  if (selectedChildId.value === null) { message("Choisissez d'abord un élève dans la liste, puis revenez sur la photo."); return; }
  const img = photoImg.value;
  const ratio = img?.naturalWidth && img.naturalHeight ? img.naturalWidth / img.naturalHeight : 1;
  const height = 0.2, width = Math.min(0.9, (height / ratio) * 0.8);
  const tag: Tag = { id: crypto.randomUUID(), childId: selectedChildId.value, x: (1 - width) / 2, y: (1 - height) / 2, width, height };
  current.value.tags.push(tag);
  await nextTick();
  imageFrame.value?.querySelector<HTMLElement>(`[data-tag="${tag.id}"]`)?.focus();
}
const arrowKeys: Record<string, [number, number]> = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };
function tagKey(event: KeyboardEvent, tag: Tag) {
  const arrow = arrowKeys[event.key];
  if (event.key === "Delete" || event.key === "Backspace") {
    removeTag(tag.id);
    imageFrame.value?.focus();
  } else if (arrow && event.shiftKey) {
    tag.width = clamp(tag.width + arrow[0] * KEY_STEP, MIN_TAG_SIZE, 1 - tag.x);
    tag.height = clamp(tag.height + arrow[1] * KEY_STEP, MIN_TAG_SIZE, 1 - tag.y);
  } else if (arrow) {
    tag.x = clamp(tag.x + arrow[0] * KEY_STEP, 0, 1 - tag.width);
    tag.y = clamp(tag.y + arrow[1] * KEY_STEP, 0, 1 - tag.height);
  } else return;
  event.preventDefault();
  event.stopPropagation();
}
function tagLabel(tag: Tag) {
  return `Cadre de ${childById(tag.childId)?.name ?? "élève retiré"}${isMasked(tag) ? ", flouté pour les autres familles" : ""}`;
}

function reassignTag(tag: Tag) { if (selectedChildId.value !== null) tag.childId = selectedChildId.value; }

async function removePhoto() {
  const photo = current.value;
  if (!photo) return;
  if (photo.tags.length && !await ask(`Retirer « ${photo.name} » de la session ? Ses cadres seront perdus. Le fichier d’origine n’est pas supprimé.`,
    { title: "Retirer la photo", kind: "warning", okLabel: "Retirer", cancelLabel: "Annuler" })) return;
  URL.revokeObjectURL(photo.url);
  photos.value.splice(selected.value, 1);
  selected.value = Math.min(selected.value, Math.max(0, photos.value.length - 1));
  focusOn(photos.value.length ? ".image-frame" : "#ajouter-photos");
}

// Zoom : garde sous le pointeur (ou au centre) le même point de l'image.
async function setZoom(value: number, anchor?: { x: number; y: number }) {
  const box = canvasBox.value, frame = imageFrame.value, img = photoImg.value;
  if (!box || !frame || !img) return;
  if (zoom.value === 1) baseWidth.value = img.getBoundingClientRect().width;
  const next = value < 1.05 ? 1 : Math.min(MAX_ZOOM, value);
  const boxRect = box.getBoundingClientRect(), frameRect = frame.getBoundingClientRect();
  const point = anchor ?? { x: boxRect.left + boxRect.width / 2, y: boxRect.top + boxRect.height / 2 };
  const fx = (point.x - frameRect.left) / frameRect.width, fy = (point.y - frameRect.top) / frameRect.height;
  zoom.value = next;
  await nextTick();
  box.scrollLeft = frame.offsetLeft + fx * frame.offsetWidth - (point.x - boxRect.left);
  box.scrollTop = frame.offsetTop + fy * frame.offsetHeight - (point.y - boxRect.top);
}
function wheelZoom(event: WheelEvent) {
  if (!event.ctrlKey && !event.metaKey) return;
  event.preventDefault();
  setZoom(zoom.value * (event.deltaY < 0 ? 1.2 : 1 / 1.2), { x: event.clientX, y: event.clientY });
}

function tagStyle(tag: Pick<Tag, "x" | "y" | "width" | "height">) {
  return { left: `${tag.x * 100}%`, top: `${tag.y * 100}%`, width: `${tag.width * 100}%`, height: `${tag.height * 100}%` };
}
function canvasBlob(canvas: HTMLCanvasElement) { return new Promise<Blob>((resolve, reject) => canvas.toBlob((blob) => blob ? resolve(blob) : reject(new Error("Encodage JPEG impossible.")), "image/jpeg", 0.92)); }
async function toBase64(blob: Blob) { return (await new Promise<string>((resolve, reject) => { const reader = new FileReader(); reader.onload = () => resolve(String(reader.result).split(",")[1]); reader.onerror = () => reject(reader.error); reader.readAsDataURL(blob); })); }

async function renderFor(photo: Photo, recipient: Child) {
  const bitmap = await createImageBitmap(photo.file);
  const canvas = document.createElement("canvas");
  canvas.width = bitmap.width; canvas.height = bitmap.height;
  const ctx = canvas.getContext("2d")!;
  ctx.drawImage(bitmap, 0, 0);
  for (const tag of photo.tags) {
    if (!isMaskedFor(tag, recipient, roster.value)) continue;
    const mask = maskEllipse(tag, canvas.width, canvas.height);
    if (!mask) continue;
    const { cx, cy, rx, ry, x, y, w, h } = mask;
    if (maskMode.value === "fill") fillEllipse(ctx, cx, cy, rx, ry);
    else obscure(ctx, canvas, x, y, w, h, cx, cy, rx, ry);
  }
  bitmap.close();
  return canvasBlob(canvas);
}

function fillEllipse(ctx: CanvasRenderingContext2D, cx: number, cy: number, rx: number, ry: number) {
  ctx.save();
  ctx.fillStyle = "#8a8f8c";
  ctx.beginPath(); ctx.ellipse(cx, cy, rx, ry, 0, 0, Math.PI * 2); ctx.fill();
  ctx.restore();
}

// Floutage sans ctx.filter (ignoré par WebKit/macOS) : réduction progressive de la zone
// jusqu’à ~6 cellules sur le petit côté, puis agrandissement lissé, limité à l’ellipse du cadre. Aucun détail du visage ne subsiste.
function obscure(ctx: CanvasRenderingContext2D, source: HTMLCanvasElement, x: number, y: number, w: number, h: number, cx: number, cy: number, rx: number, ry: number) {
  const cells = 6;
  const targetW = Math.max(1, Math.round(w * cells / Math.min(w, h)));
  const targetH = Math.max(1, Math.round(h * cells / Math.min(w, h)));
  let current = document.createElement("canvas");
  current.width = w; current.height = h;
  current.getContext("2d")!.drawImage(source, x, y, w, h, 0, 0, w, h);
  while (current.width > targetW || current.height > targetH) {
    const next = document.createElement("canvas");
    next.width = Math.max(targetW, Math.ceil(current.width / 2));
    next.height = Math.max(targetH, Math.ceil(current.height / 2));
    const nextCtx = next.getContext("2d")!;
    nextCtx.imageSmoothingEnabled = true; nextCtx.imageSmoothingQuality = "high";
    nextCtx.drawImage(current, 0, 0, next.width, next.height);
    current.width = 0; current = next;
  }
  ctx.save();
  ctx.beginPath(); ctx.ellipse(cx, cy, rx, ry, 0, 0, Math.PI * 2); ctx.clip();
  ctx.imageSmoothingEnabled = true; ctx.imageSmoothingQuality = "high";
  ctx.drawImage(current, 0, 0, current.width, current.height, x, y, w, h);
  ctx.restore();
  current.width = 0;
}

async function exportPhotos() {
  if (!photos.value.length || !roster.value.length) return;
  const jobs = exportJobs(roster.value, photos.value);
  if (!jobs.length) { message("Aucune photo n’est attribuée à un élève de la classe."); return; }
  const recipients = new Set(jobs.map((job) => job.recipient.id)).size;
  const untagged = photos.value.length - taggedPhotos.value;
  const confirmed = await ask(`${jobs.length} photo(s) seront exportées pour ${recipients} élève(s)${untagged ? `. ${untagged} photo(s) sans cadre ne seront pas exportée(s)` : ""}.

Avant d’exporter, vérifiez que chaque enfant non autorisé visible sur les photos est bien encadré, y compris au second plan. Pour un enfant hors de la classe, utilisez « Visage inconnu ».`,
    { title: "Exporter les photos", okLabel: "Choisir le dossier", cancelLabel: "Revoir les photos" });
  if (!confirmed) return;
  const destination = await open({ directory: true, multiple: false, title: "Choisir où créer le dossier d’export" });
  if (!destination || Array.isArray(destination)) return;
  const signature = tagSignature.value;
  working.value = true;
  exportProgress.value = { done: 0, total: jobs.length };
  try {
    const exportFolder = await invoke<string>("prepare_export_folder", { destination, name: exportFolderName(new Date()) });
    for (const { recipient, photo, index } of jobs) {
      const jpeg = await renderFor(photo, recipient);
      const numberedName = `${String(index + 1).padStart(3, "0")}-${jpegName(photo.name)}`;
      await invoke("write_export_image", { destination, image: { exportFolder, childFolder: cleanFolderName(recipient.name), fileName: numberedName, jpegBase64: await toBase64(jpeg) } });
      exportProgress.value.done++;
    }
    exportedSignature.value = signature;
    message(`${jobs.length} photo(s) exportée(s) dans « ${exportFolder} ». Les métadonnées ont été retirées.`);
  } catch (error) { message(`Export interrompu : ${errorText(error)}`); } finally { working.value = false; exportProgress.value = null; focusOn("#exporter"); }
}

async function discardSession() {
  if (!await ask("Retirer définitivement les photos de cette session ? Les fichiers d’origine ne seront pas supprimés.",
    { title: "Effacer la session", kind: "warning", okLabel: "Effacer", cancelLabel: "Annuler" })) return;
  photos.value.forEach((photo) => URL.revokeObjectURL(photo.url));
  photos.value = []; selected.value = 0; message("Session effacée. Aucune photo n’a été conservée par l’application.");
  focusOn("#ajouter-photos");
}
onBeforeUnmount(() => { window.removeEventListener("keydown", onKey); unlistenClose?.(); photos.value.forEach((photo) => URL.revokeObjectURL(photo.url)); });
</script>

<template>
  <div class="app" :class="{ locked: !unlocked }">
    <!-- Liens d'évitement (RGAA 12.7), visibles à la prise de focus. -->
    <nav class="skip-links" aria-label="Accès rapide">
      <template v-if="unlocked">
        <a href="#zone-photo" @click.prevent="focusOn(current ? '.image-frame' : '#zone-photo')">Aller à la photo</a>
        <a href="#eleves-titre" @click.prevent="focusOn('#eleves-titre')">Aller à la liste des élèves</a>
      </template>
      <a v-else href="#contenu" @click.prevent="focusOn('#contenu')">Aller au contenu</a>
    </nav>
    <header class="topbar">
      <div class="brand"><img :src="iconUrl" alt="" /><div><h1>À chacun sa photo</h1><p>Préparez les photos à remettre aux familles.</p></div></div>
      <div class="actions">
        <template v-if="unlocked">
        <button id="ajouter-photos" type="button" :class="photos.length ? 'secondary' : 'primary'" :disabled="working" @click="photoInput?.click()">Ajouter des photos</button>
        <input ref="photoInput" type="file" accept="image/jpeg,image/png,image/heic,.heic,.heif" multiple hidden @change="importPhotos" />
        <select v-model="maskMode" class="mask-mode" aria-label="Masquage des visages non autorisés" :disabled="working">
          <option value="blur">Masquage : flou</option>
          <option value="fill">Masquage : aplat opaque</option>
        </select>
        <button id="exporter" class="primary" :disabled="working || !photos.length || !roster.length" @click="exportPhotos">Exporter par élève</button>
        <button class="ghost" :disabled="!photos.length" @click="discardSession">Effacer la session</button>
        </template>
        <button type="button" class="ghost" @click="aboutDialog?.showModal()">À propos</button>
      </div>
    </header>
    <div v-if="exportProgress" class="export-progress">
      <label for="export-progress">Export en cours : {{ exportProgress.done }} / {{ exportProgress.total }} photo(s)</label>
      <progress id="export-progress" :value="exportProgress.done" :max="exportProgress.total || 1"></progress>
    </div>
    <!-- Zone d'état toujours présente (RGAA 7.5) : seul son texte change, pour être annoncée par les lecteurs d'écran. -->
    <p class="notice" :class="{ 'sr-only': exportProgress }" role="status">{{ exportProgress ? `Export en cours de ${exportProgress.total} photo(s)…` : notice }}</p>

    <main id="contenu" class="content" tabindex="-1">
    <section v-if="!unlocked" class="unlock" :class="{ wide: lockMode === 'setup' || lockMode === 'unlock' }">
      <template v-if="lockMode === 'legacy'"><h2>Une ancienne classe ne peut pas être ouverte</h2><p>Elle peut provenir de la version d’essai qui utilisait une ancienne protection. Vous pouvez commencer une nouvelle classe : l’ancien fichier sera archivé, sans être supprimé.</p><button class="primary" :disabled="working" @click="startNewClass">Commencer une nouvelle classe</button></template>
      <form v-else-if="lockMode === 'setup'" class="passphrase" @submit.prevent="submitPassphrase">
        <h2>Protégez la liste de vos élèves</h2>
        <p>D’habitude, l’application range toute seule la clé qui protège la liste de vos élèves dans le <strong>coffre-fort</strong> de l’ordinateur (un espace sécurisé prévu par le système). Cet ordinateur n’en propose pas : c’est fréquent sous Linux.</p>
        <p>Choisissez donc une <strong>phrase de passe</strong>. Elle sert de clé : la liste des élèves et les accords des familles sont enregistrés sous une forme illisible (chiffrée). Sans cette phrase, personne ne peut les lire, même en copiant le fichier.</p>
        <ul class="passphrase-tips">
          <li>Elle vous sera demandée <strong>à chaque ouverture</strong> de l’application.</li>
          <li>Choisissez quelques mots faciles à retenir pour vous, par exemple « le chat dort sur le radiateur » (8 caractères minimum).</li>
          <li>Notez-la en lieu sûr. Si vous l’oubliez, la liste ne pourra pas être récupérée : il faudra ressaisir la classe. Vos photos ne sont pas concernées, l’application ne les enregistre jamais.</li>
        </ul>
        <label for="lock-passphrase">Phrase de passe (8 caractères minimum)</label>
        <input id="lock-passphrase" v-model="lockPassphrase" :type="showLockPassphrase ? 'text' : 'password'" autocomplete="new-password" :aria-invalid="!!errorFor('lock-passphrase') || undefined" :aria-describedby="errorFor('lock-passphrase') ? 'lock-passphrase-error' : undefined" @input="clearFieldError('lock-passphrase')" />
        <p v-if="errorFor('lock-passphrase')" id="lock-passphrase-error" class="field-error">{{ errorFor('lock-passphrase') }}</p>
        <label for="lock-confirm">Retapez-la pour vérifier</label>
        <input id="lock-confirm" v-model="lockPassphraseConfirm" :type="showLockPassphrase ? 'text' : 'password'" autocomplete="new-password" :aria-invalid="!!errorFor('lock-confirm') || undefined" :aria-describedby="errorFor('lock-confirm') ? 'lock-confirm-error' : undefined" @input="clearFieldError('lock-confirm')" />
        <p v-if="errorFor('lock-confirm')" id="lock-confirm-error" class="field-error">{{ errorFor('lock-confirm') }}</p>
        <label class="consent-field"><input v-model="showLockPassphrase" type="checkbox" /><span>Afficher ce que je tape</span></label>
        <button class="primary" :disabled="working || !lockPassphrase || !lockPassphraseConfirm">Créer ma classe protégée</button>
      </form>
      <form v-else-if="lockMode === 'unlock'" class="passphrase" @submit.prevent="submitPassphrase">
        <h2>Votre classe</h2>
        <p>Sur cet ordinateur, la liste de vos élèves est protégée par la phrase de passe que vous avez choisie lors de la création de la classe.</p>
        <label for="lock-passphrase">Phrase de passe</label>
        <input id="lock-passphrase" v-model="lockPassphrase" :type="showLockPassphrase ? 'text' : 'password'" autocomplete="current-password" autofocus :aria-invalid="!!errorFor('lock-passphrase') || undefined" :aria-describedby="errorFor('lock-passphrase') ? 'lock-passphrase-error' : undefined" @input="clearFieldError('lock-passphrase')" />
        <p v-if="errorFor('lock-passphrase')" id="lock-passphrase-error" class="field-error">{{ errorFor('lock-passphrase') }}</p>
        <label class="consent-field"><input v-model="showLockPassphrase" type="checkbox" /><span>Afficher ce que je tape</span></label>
        <button class="primary" :disabled="working || !lockPassphrase">Ouvrir ma classe</button>
        <button type="button" class="ghost" :disabled="working" @click="forgotPassphrase">J’ai oublié ma phrase de passe</button>
      </form>
      <template v-else><h2>Votre classe</h2><p>La liste des élèves est protégée automatiquement par cet ordinateur. Les photos, elles, ne sont jamais enregistrées par l’application.</p><button class="primary" :disabled="working" @click="openRoster">Ouvrir ma classe</button></template>
    </section>

    <div v-else class="workspace">
      <aside class="register" aria-label="Élèves">
        <h2 id="eleves-titre" tabindex="-1">Élèves <span>{{ roster.length }}</span></h2>
        <form class="child-form" @submit.prevent="addChild">
          <label for="child-name" class="field-label">Prénom et nom de l’élève</label>
          <input id="child-name" v-model="childName" autocomplete="off" :aria-invalid="!!errorFor('child-name') || undefined" :aria-describedby="errorFor('child-name') ? 'child-name-error' : undefined" @input="clearFieldError('child-name')" />
          <p v-if="errorFor('child-name')" id="child-name-error" class="field-error">{{ errorFor('child-name') }}</p>
          <label class="consent-field"><input v-model="childAllowed" type="checkbox" /><span>Diffusion autorisée par la famille</span></label>
          <button class="secondary" :disabled="!childName.trim()">Ajouter l’élève</button>
          <p class="form-hint">Sans autorisation cochée, son visage est flouté dans les dossiers des autres élèves.</p>
        </form>
        <p v-if="roster.length" class="register-hint">Cliquez sur un élève, puis encadrez son visage sur la photo.</p>
        <ul class="pupils">
          <li>
            <button type="button" class="pupil unknown masked" :class="{ active: selectedChildId === UNKNOWN_ID }" :aria-pressed="selectedChildId === UNKNOWN_ID" title="Pour un enfant hors de la classe ou un visage à ne jamais montrer" @click="toggleChild(UNKNOWN_ID)">
              <span class="pupil-name">Visage inconnu</span>
              <span class="pupil-meta"><span class="status">toujours flouté</span></span>
            </button>
          </li>
          <li v-for="child in roster" :key="child.id">
            <form v-if="editingChild?.id === child.id" class="child-edit" @submit.prevent="saveEditChild">
              <label for="edit-name" class="field-label">Prénom et nom</label>
              <input id="edit-name" v-model="editingChild.name" autocomplete="off" :aria-invalid="!!errorFor('edit-name') || undefined" :aria-describedby="errorFor('edit-name') ? 'edit-name-error' : undefined" @input="clearFieldError('edit-name')" />
              <p v-if="errorFor('edit-name')" id="edit-name-error" class="field-error">{{ errorFor('edit-name') }}</p>
              <label class="consent-field"><input v-model="editingChild.allowsDistribution" type="checkbox" /><span>Diffusion autorisée par la famille</span></label>
              <div class="child-edit-actions">
                <button class="secondary" :disabled="!editingChild.name.trim()">Enregistrer</button>
                <button type="button" class="ghost" @click="stopEditChild(child.id)">Annuler</button>
                <button type="button" class="ghost danger" @click="deleteChild(child)">Supprimer</button>
              </div>
            </form>
            <template v-else>
              <button type="button" class="pupil" :class="{ active: child.id === selectedChildId, masked: !child.allowsDistribution }" :aria-pressed="child.id === selectedChildId" @click="toggleChild(child.id)">
                <span class="pupil-name">{{ child.name }}</span>
                <span class="pupil-meta"><span v-if="photos.length" class="count">{{ photoCount(child.id) }} photo{{ photoCount(child.id) > 1 ? "s" : "" }}</span><span v-if="!child.allowsDistribution" class="status">masqué</span></span>
              </button>
              <button type="button" class="edit-child" :data-edit="child.id" :aria-label="`Modifier ${child.name}`" title="Modifier le nom ou l’accord" @click="startEditChild(child)">✎</button>
            </template>
          </li>
        </ul>
        <p v-if="!roster.length" class="register-empty">Aucun élève pour l’instant. Ajoutez la classe ci-dessus : seuls le nom et l’accord de diffusion sont enregistrés.</p>
        <details class="roster-tools">
          <summary>Gérer la classe</summary>
          <button type="button" class="secondary" @click="rosterInput?.click()">Importer une liste (CSV, TXT)</button>
          <input ref="rosterInput" type="file" accept=".csv,.txt,text/csv,text/plain" hidden @change="importRoster" />
          <p class="form-hint">Un nom par ligne, ou un export tableur avec les colonnes « Nom » et « Prénom ». Les élèves importés sont masqués par défaut.</p>
          <fieldset>
            <legend>Changer d’ordinateur</legend>
            <label for="backup-passphrase" class="field-label">Phrase de passe de la sauvegarde (8 caractères minimum)</label>
            <input id="backup-passphrase" v-model="passphrase" type="password" autocomplete="off" :aria-invalid="!!errorFor('backup-passphrase') || undefined" :aria-describedby="errorFor('backup-passphrase') ? 'backup-passphrase-error' : undefined" @input="clearFieldError('backup-passphrase')" />
            <p v-if="errorFor('backup-passphrase')" id="backup-passphrase-error" class="field-error">{{ errorFor('backup-passphrase') }}</p>
            <div class="tool-row">
              <button type="button" class="secondary" :disabled="!roster.length" @click="backupRoster">Sauvegarder</button>
              <button type="button" class="secondary" @click="restoreRoster">Restaurer</button>
            </div>
            <p class="form-hint">La sauvegarde est chiffrée avec cette phrase de passe. Sans elle, personne ne peut l’ouvrir, pas même vous.</p>
          </fieldset>
          <button type="button" class="ghost danger" :disabled="!roster.length" @click="clearRoster">Nouvelle année : effacer la classe</button>
        </details>
      </aside>

      <section id="zone-photo" ref="stageSection" class="stage" aria-labelledby="zone-photo-titre" tabindex="-1">
        <h2 id="zone-photo-titre" class="sr-only">Photo en cours</h2>
        <template v-if="current">
          <div class="photo-nav">
            <button class="nav" :disabled="selected === 0" aria-label="Photo précédente" @click="selected--">‹</button>
            <strong :title="current.name">{{ current.name }}</strong>
            <span class="position">{{ selected + 1 }} / {{ photos.length }}</span>
            <button class="nav" :disabled="selected >= photos.length - 1" aria-label="Photo suivante" @click="selected++">›</button>
            <div class="zoom" role="group" aria-label="Zoom">
              <button class="nav" :disabled="zoom <= 1" aria-label="Dézoomer" title="Dézoomer (−)" @click="setZoom(zoom / 1.5)">−</button>
              <button class="nav zoom-level" :disabled="zoom === 1" :aria-label="`${Math.round(zoom * 100)} %, ajuster à la fenêtre`" title="Ajuster à la fenêtre (0)" @click="zoom = 1">{{ Math.round(zoom * 100) }} %</button>
              <button class="nav" :disabled="zoom >= MAX_ZOOM" aria-label="Zoomer" title="Zoomer (+ ou Ctrl + molette)" @click="setZoom(zoom * 1.5)">+</button>
            </div>
            <button class="nav remove-photo" title="Retirer cette photo de la session (le fichier d’origine n’est pas supprimé)" @click="removePhoto">Retirer la photo</button>
          </div>
          <div ref="canvasBox" class="canvas" :class="{ zoomed: zoom > 1 }" @wheel="wheelZoom">
            <div ref="imageFrame" class="image-frame" :class="{ armed: selectedChildId !== null }" tabindex="0" role="group" :aria-label="`Photo ${current.name}`" aria-describedby="keyboard-help" @keydown.enter.self.prevent="addTagAtCenter" @pointerdown="beginTag" @pointermove="moveTag" @pointerup="finishTag" @pointercancel="finishTag">
              <img ref="photoImg" :src="current.url" :alt="current.name" :style="imgStyle" draggable="false" />
              <div v-for="tag in displayTags" :key="tag.id" class="tag-box" :style="tagStyle(tag)">
                <div class="tag" :class="{ masked: isMasked(tag), editing: editing?.tag.id === tag.id }" :data-tag="tag.id" tabindex="0" role="group" aria-roledescription="cadre" :aria-label="tagLabel(tag)" aria-describedby="keyboard-help" @keydown="tagKey($event, tag)" title="Glisser pour déplacer, tirer un coin pour redimensionner" @pointerdown.stop="beginEdit($event, tag, 'move')"><div v-for="handle in handles" :key="handle" class="handle" :class="handle" aria-hidden="true" @pointerdown.stop="beginEdit($event, tag, handle)"></div><span aria-hidden="true">{{ childById(tag.childId)?.name ?? "Élève retiré" }}</span></div>
                <button v-if="selectedChild && selectedChildId !== tag.childId" type="button" class="tag-action reassign" :aria-label="`Attribuer ce cadre à ${selectedChild.name}`" :title="`Attribuer ce cadre à ${selectedChild.name}`" @pointerdown.stop @click.stop="reassignTag(tag)">⇄</button>
                <button type="button" class="tag-action" :aria-label="`Retirer le cadre de ${childById(tag.childId)?.name ?? 'élève retiré'}`" title="Retirer ce cadre" @pointerdown.stop @click.stop="removeTag(tag.id); imageFrame?.focus()">×</button>
              </div>
              <div v-if="draft" class="tag draft" :class="{ masked: !selectedChild?.allowsDistribution }" :style="tagStyle(draft)"></div>
            </div>
          </div>
          <p id="keyboard-help" class="sr-only">Au clavier : Entrée sur la photo ajoute un cadre pour l’élève choisi. Sur un cadre, flèches pour le déplacer, Majuscule et flèches pour le redimensionner, Suppr pour le retirer.</p>
          <p class="hint">
            <template v-if="selectedChildId === UNKNOWN_ID">Encadrez un visage à flouter dans tous les dossiers (enfant d’une autre classe, visiteur…).</template>
            <template v-else-if="selectedChild">Encadrez le visage de <strong>{{ selectedChild.name }}</strong>.<template v-if="!selectedChild.allowsDistribution"> Il sera flouté dans les dossiers des autres élèves.</template></template>
            <template v-else>Choisissez un élève dans la liste pour commencer à encadrer.</template>
          </p>
          <nav class="strip" aria-label="Photos de la session">
            <span class="strip-summary">{{ taggedPhotos }} sur {{ photos.length }} identifiée{{ taggedPhotos > 1 ? "s" : "" }}</span>
            <ol>
              <li v-for="(photo, index) in photos" :key="photo.id"><button type="button" :class="{ current: index === selected, done: photo.tags.length }" :aria-current="index === selected ? 'true' : undefined" :aria-label="`Photo ${index + 1}${photo.tags.length ? ', identifiée' : ''}`" @click="selected = index"><span v-if="photo.tags.length" class="done-mark" aria-hidden="true">✓</span>{{ index + 1 }}</button></li>
            </ol>
          </nav>
        </template>
        <div v-else class="empty">
          <h2>Ajoutez les photos de la sortie, du spectacle ou de la classe</h2>
          <p>Elles restent uniquement dans la mémoire de cette session, jusqu’à l’export ou l’effacement. Formats acceptés : JPEG, PNG, HEIC.</p>
          <button type="button" class="primary" :disabled="working" @click="photoInput?.click()">Ajouter des photos</button>
        </div>
      </section>
    </div>
    </main>
    <dialog ref="aboutDialog" class="about" aria-labelledby="about-title">
      <form method="dialog" class="about-close"><button class="ghost" aria-label="Fermer">×</button></form>
      <h2 id="about-title">À chacun sa photo <span>version {{ version }}</span></h2>
      <p>Trier les photos de classe par élève, en floutant les enfants dont la famille n’a pas autorisé la diffusion.</p>
      <h3>Vos données</h3>
      <ul>
        <li>L’application fonctionne <strong>sans Internet</strong> : aucune connexion, aucun compte, aucune statistique d’utilisation.</li>
        <li>Seule la liste des élèves (nom et accord de diffusion) est enregistrée, chiffrée, sur cet ordinateur.</li>
        <li>Les photos ne sont jamais copiées par l’application : elles restent en mémoire le temps de la session, puis sont écrites uniquement dans le dossier d’export que vous choisissez, sans leurs métadonnées (date, lieu GPS…).</li>
        <li>L’application aide à respecter les choix des familles mais ne détecte pas les visages : c’est à vous d’encadrer chaque enfant non autorisé visible, y compris au second plan.</li>
      </ul>
      <h3>Utiliser le clavier</h3>
      <ul>
        <li><kbd>←</kbd> <kbd>→</kbd> : photo précédente ou suivante ; <kbd>+</kbd> <kbd>−</kbd> <kbd>0</kbd> : zoom sur la photo (quand la photo ou ses boutons ont le focus).</li>
        <li><kbd>Ctrl</kbd> (<kbd>⌘</kbd> sur Mac) + <kbd>=</kbd> ou <kbd>−</kbd> : agrandir ou réduire le texte de l’application ; + <kbd>0</kbd> : taille normale.</li>
        <li>Choisissez un élève, placez-vous sur la photo avec <kbd>Tab</kbd> puis appuyez sur <kbd>Entrée</kbd> pour ajouter un cadre au centre.</li>
        <li>Sur un cadre : flèches pour le déplacer, <kbd>Maj</kbd> + flèches pour le redimensionner, <kbd>Suppr</kbd> pour le retirer.</li>
      </ul>
      <h3>Mises à jour</h3>
      <p>L’application ne se met jamais à jour toute seule (elle ne se connecte pas à Internet). Consultez de temps en temps la page où vous l’avez téléchargée pour installer la dernière version.</p>
      <h3>Licence</h3>
      <p>Logiciel libre distribué sous licence EUPL-1.2 (licence publique de l’Union européenne). Vous pouvez l’utiliser, l’étudier, le modifier et le redistribuer.</p>
      <details>
        <summary>Composants tiers et leurs licences</summary>
        <pre lang="en">{{ thirdPartyNotices }}</pre>
      </details>
    </dialog>
  </div>
</template>
