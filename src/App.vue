<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ask, open, save } from "@tauri-apps/plugin-dialog";
import iconUrl from "../src-tauri/icons/icon.png";

type Child = { id: number; name: string; allowsDistribution: boolean };
type Tag = { id: string; childId: number; x: number; y: number; width: number; height: number };
type Photo = { id: string; name: string; file: File; url: string; tags: Tag[] };

const roster = ref<Child[]>([]);
const photos = ref<Photo[]>([]);
const selected = ref(0);
const unlocked = ref(false);
const legacyRoster = ref(false);
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
const canvasBox = ref<HTMLElement>();
const photoImg = ref<HTMLImageElement>();
const zoom = ref(1);
const baseWidth = ref(0);
const MAX_ZOOM = 6;
// Visage à flouter partout (enfant d'une autre classe, passant…) : jamais de dossier, jamais d'accord.
const UNKNOWN_ID = 0;
const unknownChild: Child = { id: UNKNOWN_ID, name: "Visage inconnu", allowsDistribution: false };
const drawing = ref<{ x: number; y: number } | null>(null);
const draft = ref<{ x: number; y: number; width: number; height: number } | null>(null);
type Handle = "move" | "nw" | "ne" | "sw" | "se";
const handles: Handle[] = ["nw", "ne", "sw", "se"];
const MIN_TAG_SIZE = 0.015;
const editing = ref<{ tag: Tag; handle: Handle; start: { x: number; y: number }; origin: { x: number; y: number; width: number; height: number } } | null>(null);

const current = computed(() => photos.value[selected.value]);
const childById = (id: number) => id === UNKNOWN_ID ? unknownChild : roster.value.find((child) => child.id === id);
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
  if (["INPUT", "SELECT", "TEXTAREA"].includes(target.tagName)) return;
  if (event.key === "ArrowLeft" && selected.value > 0) selected.value--;
  if (event.key === "ArrowRight" && selected.value < photos.value.length - 1) selected.value++;
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

function message(value: string) { notice.value = value; }

async function openRoster() {
  working.value = true;
  try {
    await invoke("open_roster");
    roster.value = await invoke<Child[]>("list_children");
    unlocked.value = true;
    legacyRoster.value = false;
    message("La classe est prête.");
  } catch (error) {
    if (error === "LEGACY_ROSTER") {
      legacyRoster.value = true;
      message("La classe enregistrée sur cet ordinateur ne peut pas être ouverte.");
    } else {
      message(String(error));
    }
  } finally { working.value = false; }
}

async function startNewClass() {
  working.value = true;
  try {
    await invoke("archive_legacy_roster");
    await openRoster();
    message("Nouvelle classe prête. L’ancien fichier a été archivé, sans être supprimé.");
  } catch (error) { message(String(error)); } finally { working.value = false; }
}

async function reloadRoster() { roster.value = await invoke<Child[]>("list_children"); }

async function addChild() {
  if (!childName.value.trim()) return;
  try {
    await invoke("save_child", { name: childName.value, allowsDistribution: childAllowed.value });
    await reloadRoster();
    childName.value = "";
    childAllowed.value = false;
  } catch (error) { message(errorText(error)); }
}

function startEditChild(child: Child) { editingChild.value = { ...child }; }
async function saveEditChild() {
  const edit = editingChild.value;
  if (!edit || !edit.name.trim()) return;
  try {
    await invoke("update_child", { id: edit.id, name: edit.name, allowsDistribution: edit.allowsDistribution });
    await reloadRoster();
    editingChild.value = null;
  } catch (error) { message(errorText(error)); }
}

// Les cadres d'un élève retiré de la classe restent floutés : ils deviennent des visages inconnus.
function forgetChildren(ids: number[]) {
  for (const photo of photos.value) for (const tag of photo.tags) if (ids.includes(tag.childId)) tag.childId = UNKNOWN_ID;
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
  } catch (error) { message(errorText(error)); }
}

function decodeText(buffer: ArrayBuffer) {
  try { return new TextDecoder("utf-8", { fatal: true }).decode(buffer); } catch { return new TextDecoder("windows-1252").decode(buffer); }
}

// Liste d'élèves : un nom par ligne, ou export tableur/ONDE avec colonnes « Nom » et « Prénom ».
function parseNames(text: string): string[] {
  const lines = text.replace(/^\uFEFF/, "").split(/\r?\n/).filter((line) => line.trim());
  if (!lines.length) return [];
  const separator = [";", "\t", ","].find((candidate) => lines[0].includes(candidate));
  const cells = (line: string) => (separator ? line.split(separator) : [line]).map((cell) => cell.trim().replace(/^"(.*)"$/, "$1").trim());
  const header = cells(lines[0]).map((cell) => cell.normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase());
  const first = header.findIndex((cell) => cell.startsWith("prenom"));
  const last = header.findIndex((cell) => cell.startsWith("nom"));
  const hasHeader = first >= 0 || last >= 0;
  const names = (hasHeader ? lines.slice(1) : lines).map(cells).map((row) =>
    first >= 0 && last >= 0 ? `${row[first] ?? ""} ${row[last] ?? ""}` : row[first >= 0 ? first : last >= 0 ? last : 0] ?? "");
  return [...new Set(names.map((name) => name.replace(/\s+/g, " ").trim()).filter(Boolean))];
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
  if (passphrase.value.length < 8) { message("Choisissez d’abord une phrase de passe d’au moins 8 caractères."); return; }
  const destination = await save({ title: "Enregistrer la sauvegarde de la classe", defaultPath: `classe-${new Date().toISOString().slice(0, 10)}.sauvegarde`, filters: backupFilter });
  if (!destination) return;
  try {
    await invoke("backup_roster", { destination, passphrase: passphrase.value });
    passphrase.value = "";
    message("Sauvegarde chiffrée enregistrée. Conservez la phrase de passe : sans elle, la sauvegarde ne peut pas être rouverte.");
  } catch (error) { message(errorText(error)); }
}
async function restoreRoster() {
  if (passphrase.value.length < 8) { message("Saisissez d’abord la phrase de passe de la sauvegarde."); return; }
  const source = await open({ title: "Choisir une sauvegarde de classe", multiple: false, directory: false, filters: backupFilter });
  if (!source || Array.isArray(source)) return;
  try {
    const count = await invoke<number>("restore_roster", { source, passphrase: passphrase.value });
    passphrase.value = "";
    await reloadRoster();
    message(`${count} élève(s) restauré(s) depuis la sauvegarde.`);
  } catch (error) { message(errorText(error)); }
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
  const dx = end.x - start.x, dy = end.y - start.y;
  const clamp = (value: number, min: number, max: number) => Math.max(min, Math.min(max, value));
  if (handle === "move") {
    tag.x = clamp(o.x + dx, 0, 1 - o.width);
    tag.y = clamp(o.y + dy, 0, 1 - o.height);
    return;
  }
  let left = o.x, top = o.y, right = o.x + o.width, bottom = o.y + o.height;
  if (handle.includes("w")) left = clamp(left + dx, 0, right - MIN_TAG_SIZE);
  if (handle.includes("e")) right = clamp(right + dx, left + MIN_TAG_SIZE, 1);
  if (handle.includes("n")) top = clamp(top + dy, 0, bottom - MIN_TAG_SIZE);
  if (handle.includes("s")) bottom = clamp(bottom + dy, top + MIN_TAG_SIZE, 1);
  Object.assign(tag, { x: left, y: top, width: right - left, height: bottom - top });
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
function reassignTag(tag: Tag) { if (selectedChildId.value !== null) tag.childId = selectedChildId.value; }

async function removePhoto() {
  const photo = current.value;
  if (!photo) return;
  if (photo.tags.length && !await ask(`Retirer « ${photo.name} » de la session ? Ses cadres seront perdus. Le fichier d’origine n’est pas supprimé.`,
    { title: "Retirer la photo", kind: "warning", okLabel: "Retirer", cancelLabel: "Annuler" })) return;
  URL.revokeObjectURL(photo.url);
  photos.value.splice(selected.value, 1);
  selected.value = Math.min(selected.value, Math.max(0, photos.value.length - 1));
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
function cleanFolderName(name: string) { return name.replace(/[\\/:*?"<>|]/g, "-").trim() || "élève"; }
function jpegName(name: string) { return `${name.replace(/\.[^.]+$/, "") || "photo"}.jpg`; }
function canvasBlob(canvas: HTMLCanvasElement) { return new Promise<Blob>((resolve, reject) => canvas.toBlob((blob) => blob ? resolve(blob) : reject(new Error("Encodage JPEG impossible.")), "image/jpeg", 0.92)); }
async function toBase64(blob: Blob) { return (await new Promise<string>((resolve, reject) => { const reader = new FileReader(); reader.onload = () => resolve(String(reader.result).split(",")[1]); reader.onerror = () => reject(reader.error); reader.readAsDataURL(blob); })); }

async function renderFor(photo: Photo, recipient: Child) {
  const bitmap = await createImageBitmap(photo.file);
  const canvas = document.createElement("canvas");
  canvas.width = bitmap.width; canvas.height = bitmap.height;
  const ctx = canvas.getContext("2d")!;
  ctx.drawImage(bitmap, 0, 0);
  for (const tag of photo.tags) {
    const child = childById(tag.childId);
    if (child?.id === recipient.id || child?.allowsDistribution) continue;
    // Ellipse inscrite dans le cadre (marge de 1,5 % de chaque côté), calculée avant tout rognage aux bords.
    const cx = (tag.x + tag.width / 2) * canvas.width;
    const cy = (tag.y + tag.height / 2) * canvas.height;
    const rx = (tag.width / 2 + 0.015) * canvas.width;
    const ry = (tag.height / 2 + 0.015) * canvas.height;
    const x = Math.max(0, Math.floor(cx - rx));
    const y = Math.max(0, Math.floor(cy - ry));
    const w = Math.min(canvas.width, Math.ceil(cx + rx)) - x;
    const h = Math.min(canvas.height, Math.ceil(cy + ry)) - y;
    if (w <= 0 || h <= 0) continue;
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

function exportFolderName(date: Date) {
  const two = (value: number) => String(value).padStart(2, "0");
  return `Photos du ${two(date.getDate())}-${two(date.getMonth() + 1)}-${date.getFullYear()} à ${two(date.getHours())}h${two(date.getMinutes())}`;
}

async function exportPhotos() {
  if (!photos.value.length || !roster.value.length) return;
  const jobs = roster.value.flatMap((recipient) =>
    photos.value.filter((item) => item.tags.some((tag) => tag.childId === recipient.id)).map((photo, index) => ({ recipient, photo, index })));
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
  } catch (error) { message(`Export interrompu : ${errorText(error)}`); } finally { working.value = false; exportProgress.value = null; }
}

async function discardSession() {
  if (!await ask("Retirer définitivement les photos de cette session ? Les fichiers d’origine ne seront pas supprimés.",
    { title: "Effacer la session", kind: "warning", okLabel: "Effacer", cancelLabel: "Annuler" })) return;
  photos.value.forEach((photo) => URL.revokeObjectURL(photo.url));
  photos.value = []; selected.value = 0; message("Session effacée. Aucune photo n’a été conservée par l’application.");
}
onBeforeUnmount(() => { window.removeEventListener("keydown", onKey); unlistenClose?.(); photos.value.forEach((photo) => URL.revokeObjectURL(photo.url)); });
</script>

<template>
  <main :class="{ locked: !unlocked }">
    <header class="topbar">
      <div class="brand"><img :src="iconUrl" alt="" /><div><h1>À chacun sa photo</h1><p>Préparez les photos à remettre aux familles.</p></div></div>
      <div v-if="unlocked" class="actions">
        <label class="button" :class="photos.length ? 'secondary' : 'primary'">Ajouter des photos<input type="file" accept="image/jpeg,image/png,image/heic,.heic,.heif" multiple :disabled="working" @change="importPhotos" /></label>
        <select v-model="maskMode" class="mask-mode" aria-label="Masquage des visages non autorisés" :disabled="working">
          <option value="blur">Masquage : flou</option>
          <option value="fill">Masquage : aplat opaque</option>
        </select>
        <button class="primary" :disabled="working || !photos.length || !roster.length" @click="exportPhotos">Exporter par élève</button>
        <button class="ghost" :disabled="!photos.length" @click="discardSession">Effacer la session</button>
      </div>
    </header>
    <div v-if="exportProgress" class="export-progress" role="status">
      <label for="export-progress">Export en cours : {{ exportProgress.done }} / {{ exportProgress.total }} photo(s)</label>
      <progress id="export-progress" :value="exportProgress.done" :max="exportProgress.total || 1"></progress>
    </div>
    <p v-else-if="notice" class="notice" role="status">{{ notice }}</p>

    <section v-if="!unlocked" class="unlock">
      <template v-if="legacyRoster"><h2>Une ancienne classe ne peut pas être ouverte</h2><p>Elle peut provenir de la version d’essai qui utilisait une ancienne protection. Vous pouvez commencer une nouvelle classe : l’ancien fichier sera archivé, sans être supprimé.</p><button class="primary" :disabled="working" @click="startNewClass">Commencer une nouvelle classe</button></template>
      <template v-else><h2>Votre classe</h2><p>La liste des élèves est protégée automatiquement par cet ordinateur. Les photos, elles, ne sont jamais enregistrées par l’application.</p><button class="primary" :disabled="working" @click="openRoster">Ouvrir ma classe</button></template>
    </section>

    <div v-else class="workspace">
      <aside class="register" aria-label="Élèves">
        <h2>Élèves <span>{{ roster.length }}</span></h2>
        <form class="child-form" @submit.prevent="addChild">
          <input v-model="childName" placeholder="Prénom et nom" aria-label="Prénom et nom" />
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
              <input v-model="editingChild.name" :aria-label="`Nom de ${child.name}`" />
              <label class="consent-field"><input v-model="editingChild.allowsDistribution" type="checkbox" /><span>Diffusion autorisée par la famille</span></label>
              <div class="child-edit-actions">
                <button class="secondary" :disabled="!editingChild.name.trim()">Enregistrer</button>
                <button type="button" class="ghost" @click="editingChild = null">Annuler</button>
                <button type="button" class="ghost danger" @click="deleteChild(child)">Supprimer</button>
              </div>
            </form>
            <template v-else>
              <button type="button" class="pupil" :class="{ active: child.id === selectedChildId, masked: !child.allowsDistribution }" :aria-pressed="child.id === selectedChildId" @click="toggleChild(child.id)">
                <span class="pupil-name">{{ child.name }}</span>
                <span class="pupil-meta"><span v-if="photos.length" class="count">{{ photoCount(child.id) }} photo{{ photoCount(child.id) > 1 ? "s" : "" }}</span><span v-if="!child.allowsDistribution" class="status">masqué</span></span>
              </button>
              <button type="button" class="edit-child" :aria-label="`Modifier ${child.name}`" title="Modifier le nom ou l’accord" @click="startEditChild(child)">✎</button>
            </template>
          </li>
        </ul>
        <p v-if="!roster.length" class="register-empty">Aucun élève pour l’instant. Ajoutez la classe ci-dessus : seuls le nom et l’accord de diffusion sont enregistrés.</p>
        <details class="roster-tools">
          <summary>Gérer la classe</summary>
          <label class="button secondary">Importer une liste (CSV, TXT)<input type="file" accept=".csv,.txt,text/csv,text/plain" @change="importRoster" /></label>
          <p class="form-hint">Un nom par ligne, ou un export tableur avec les colonnes « Nom » et « Prénom ». Les élèves importés sont masqués par défaut.</p>
          <fieldset>
            <legend>Changer d’ordinateur</legend>
            <input v-model="passphrase" type="password" autocomplete="off" placeholder="Phrase de passe (8 caractères min.)" aria-label="Phrase de passe de la sauvegarde" />
            <div class="tool-row">
              <button type="button" class="secondary" :disabled="!roster.length" @click="backupRoster">Sauvegarder</button>
              <button type="button" class="secondary" @click="restoreRoster">Restaurer</button>
            </div>
            <p class="form-hint">La sauvegarde est chiffrée avec cette phrase de passe. Sans elle, personne ne peut l’ouvrir, pas même vous.</p>
          </fieldset>
          <button type="button" class="ghost danger" :disabled="!roster.length" @click="clearRoster">Nouvelle année : effacer la classe</button>
        </details>
      </aside>

      <section class="stage" aria-label="Photo en cours">
        <template v-if="current">
          <div class="photo-nav">
            <button class="nav" :disabled="selected === 0" aria-label="Photo précédente" @click="selected--">‹</button>
            <strong :title="current.name">{{ current.name }}</strong>
            <span class="position">{{ selected + 1 }} / {{ photos.length }}</span>
            <button class="nav" :disabled="selected >= photos.length - 1" aria-label="Photo suivante" @click="selected++">›</button>
            <div class="zoom" role="group" aria-label="Zoom">
              <button class="nav" :disabled="zoom <= 1" aria-label="Dézoomer" title="Dézoomer (−)" @click="setZoom(zoom / 1.5)">−</button>
              <button class="nav zoom-level" :disabled="zoom === 1" title="Ajuster à la fenêtre (0)" @click="zoom = 1">{{ Math.round(zoom * 100) }} %</button>
              <button class="nav" :disabled="zoom >= MAX_ZOOM" aria-label="Zoomer" title="Zoomer (+ ou Ctrl + molette)" @click="setZoom(zoom * 1.5)">+</button>
            </div>
            <button class="nav remove-photo" title="Retirer cette photo de la session" @click="removePhoto">Retirer</button>
          </div>
          <div ref="canvasBox" class="canvas" :class="{ zoomed: zoom > 1 }" @wheel="wheelZoom">
            <div ref="imageFrame" class="image-frame" :class="{ armed: selectedChildId !== null }" @pointerdown="beginTag" @pointermove="moveTag" @pointerup="finishTag" @pointercancel="finishTag">
              <img ref="photoImg" :src="current.url" :alt="current.name" :style="imgStyle" draggable="false" />
              <div v-for="tag in displayTags" :key="tag.id" class="tag" :class="{ masked: isMasked(tag), editing: editing?.tag.id === tag.id }" :style="tagStyle(tag)" title="Glisser pour déplacer, tirer un coin pour redimensionner" @pointerdown.stop="beginEdit($event, tag, 'move')"><div v-for="handle in handles" :key="handle" class="handle" :class="handle" aria-hidden="true" @pointerdown.stop="beginEdit($event, tag, handle)"></div><span>{{ childById(tag.childId)?.name ?? "Élève retiré" }}</span><button v-if="selectedChild && selectedChildId !== tag.childId" class="reassign" :aria-label="`Attribuer ce cadre à ${selectedChild.name}`" :title="`Attribuer ce cadre à ${selectedChild.name}`" @pointerdown.stop @click.stop="reassignTag(tag)">⇄</button><button :aria-label="`Retirer le cadre de ${childById(tag.childId)?.name}`" title="Retirer ce cadre" @pointerdown.stop @click.stop="removeTag(tag.id)">×</button></div>
              <div v-if="draft" class="tag draft" :class="{ masked: !selectedChild?.allowsDistribution }" :style="tagStyle(draft)"></div>
            </div>
          </div>
          <p class="hint">
            <template v-if="selectedChildId === UNKNOWN_ID">Encadrez un visage à flouter dans tous les dossiers (enfant d’une autre classe, visiteur…).</template>
            <template v-else-if="selectedChild">Encadrez le visage de <strong>{{ selectedChild.name }}</strong>.<template v-if="!selectedChild.allowsDistribution"> Il sera flouté dans les dossiers des autres élèves.</template></template>
            <template v-else>Choisissez un élève dans la liste pour commencer à encadrer.</template>
          </p>
          <nav class="strip" aria-label="Photos de la session">
            <span class="strip-summary">{{ taggedPhotos }} sur {{ photos.length }} identifiée{{ taggedPhotos > 1 ? "s" : "" }}</span>
            <ol>
              <li v-for="(photo, index) in photos" :key="photo.id"><button type="button" :class="{ current: index === selected, done: photo.tags.length }" :aria-current="index === selected ? 'true' : undefined" :aria-label="`Photo ${index + 1}${photo.tags.length ? ', identifiée' : ''}`" @click="selected = index">{{ index + 1 }}</button></li>
            </ol>
          </nav>
        </template>
        <div v-else class="empty">
          <h2>Ajoutez les photos de la sortie, du spectacle ou de la classe</h2>
          <p>Elles restent uniquement dans la mémoire de cette session, jusqu’à l’export ou l’effacement. Formats acceptés : JPEG, PNG, HEIC.</p>
          <label class="button primary">Ajouter des photos<input type="file" accept="image/jpeg,image/png,image/heic,.heic,.heif" multiple :disabled="working" @change="importPhotos" /></label>
        </div>
      </section>
    </div>
  </main>
</template>
