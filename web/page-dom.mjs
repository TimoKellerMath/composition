// A small DOM for testing the page without a browser (page-test.mjs): it reads web/index.html
// with a simple HTML parser and gives its elements the properties, events and selectors that
// app.js uses. Events bubble from an element up to the document; a click on a submit button
// submits its form. Nothing is drawn.

/** The elements without an end tag, and those whose content is text. */
const VOID = new Set("area base br col embed hr img input link meta source track wbr".split(" "));
const RAW = new Set(["script", "style"]);

class Text {
  constructor(data) {
    this.data = String(data);
    this.parentNode = null;
  }
  get textContent() {
    return this.data;
  }
}

/** The attribute of `data-*` that `dataset[key]` names: `fooBar` is `data-foo-bar`. */
const dataName = (key) => `data-${key.replace(/[A-Z]/g, (c) => `-${c.toLowerCase()}`)}`;

/** `element` and the elements it is in, from the inside out. */
function outward(element) {
  const elements = [];
  for (let node = element; node; node = node.parentNode) elements.push(node);
  return elements;
}

/** Takes `node` out of the element it is in, if any. */
function detach(node) {
  const siblings = node.parentNode?.childNodes ?? [];
  const at = siblings.indexOf(node);
  if (at >= 0) siblings.splice(at, 1);
  node.parentNode = null;
}

let top = null;

export class Element {
  constructor(tag, attributes = {}) {
    this.localName = tag.toLowerCase();
    this.tagName = this.localName.toUpperCase();
    this.attributes = { ...attributes };
    this.childNodes = [];
    this.parentNode = null;
    this.listeners = {};
    this.style = {};
    this.placeholder = attributes.placeholder ?? "";
    this.title = attributes.title ?? "";
    this.disabled = "disabled" in attributes;
    this.hidden = "hidden" in attributes;
    this.ownChecked = "checked" in attributes;
    this.ownValue = undefined;
    this.chosen = undefined;
    // The arrow functions below take `this`, the element.
    this.dataset = new Proxy({}, {
      get: (_, key) => (typeof key === "string" ? this.attributes[dataName(key)] : undefined),
      set: (_, key, value) => {
        this.attributes[dataName(key)] = String(value);
        return true;
      },
      has: (_, key) => dataName(key) in this.attributes,
    });
    this.classList = {
      contains: (name) => this.classes().includes(name),
      add: (...names) => {
        this.className = [...new Set([...this.classes(), ...names])].join(" ");
      },
      remove: (...names) => {
        this.className = this.classes().filter((c) => !names.includes(c)).join(" ");
      },
      toggle: (name, force) => {
        const on = force === undefined ? !this.classes().includes(name) : Boolean(force);
        if (on) this.classList.add(name);
        else this.classList.remove(name);
        return on;
      },
    };
  }

  classes() {
    return (this.attributes.class ?? "").split(/\s+/).filter(Boolean);
  }
  get className() {
    return this.attributes.class ?? "";
  }
  set className(value) {
    this.attributes.class = String(value);
  }
  get id() {
    return this.attributes.id ?? "";
  }
  get name() {
    return this.attributes.name ?? "";
  }
  get type() {
    if (this.attributes.type) return this.attributes.type.toLowerCase();
    if (this.localName === "button") return "submit";
    if (this.localName === "input") return "text";
    return "";
  }
  set type(value) {
    this.attributes.type = String(value);
  }

  get children() {
    return this.childNodes.filter((node) => node instanceof Element);
  }
  get textContent() {
    return this.childNodes.map((node) => node.textContent).join("");
  }
  set textContent(value) {
    this.replaceChildren(new Text(value ?? ""));
  }
  get innerHTML() {
    return this.html ?? this.textContent;
  }
  set innerHTML(value) {
    this.replaceChildren();
    this.html = String(value);
  }

  get checked() {
    return this.ownChecked;
  }
  set checked(value) {
    this.ownChecked = Boolean(value);
    if (this.ownChecked && this.type === "radio" && this.name) {
      for (const other of top.querySelectorAll(`input[name=${this.name}]`)) {
        if (other !== this) other.ownChecked = false;
      }
    }
  }

  get options() {
    return this.descendants().filter((element) => element.localName === "option");
  }
  get selectedIndex() {
    const options = this.options;
    if (this.chosen === null) return -1;
    if (this.chosen && options.includes(this.chosen)) return options.indexOf(this.chosen);
    const marked = options.findIndex((option) => "selected" in option.attributes);
    return marked >= 0 ? marked : (options.length ? 0 : -1);
  }
  set selectedIndex(index) {
    this.chosen = this.options[index] ?? null;
  }
  get selectedOptions() {
    const option = this.options[this.selectedIndex];
    return option ? [option] : [];
  }
  add(option) {
    this.appendChild(option);
  }
  get value() {
    if (this.localName === "select") return this.options[this.selectedIndex]?.value ?? "";
    if (this.localName === "option") return this.attributes.value ?? this.textContent.trim();
    if (this.localName === "textarea") return this.ownValue ?? this.textContent;
    if (this.ownValue !== undefined) return this.ownValue;
    if (this.attributes.value !== undefined) return this.attributes.value;
    return this.type === "checkbox" || this.type === "radio" ? "on" : "";
  }
  set value(value) {
    if (this.localName === "select") {
      this.chosen = this.options.find((option) => option.value === String(value)) ?? null;
    } else if (this.localName === "option") {
      this.attributes.value = String(value);
    } else {
      this.ownValue = String(value);
    }
  }
  get files() {
    return [];
  }
  get form() {
    let node = this.parentNode;
    while (node && node.localName !== "form") node = node.parentNode;
    return node ?? null;
  }
  get clientWidth() {
    return 800;
  }

  appendChild(node) {
    detach(node);
    node.parentNode = this;
    this.childNodes.push(node);
    return node;
  }
  append(...nodes) {
    for (const node of nodes) this.appendChild(typeof node === "string" ? new Text(node) : node);
  }
  replaceChildren(...nodes) {
    for (const node of this.childNodes) node.parentNode = null;
    this.childNodes = [];
    this.html = undefined;
    this.append(...nodes);
  }
  remove() {
    detach(this);
  }
  setAttribute(name, value) {
    this.attributes[name] = String(value);
    if (name === "disabled") this.disabled = true;
    if (name === "hidden") this.hidden = true;
  }
  getAttribute(name) {
    return this.attributes[name] ?? null;
  }

  addEventListener(type, listener) {
    (this.listeners[type] ??= []).push(listener);
  }
  /** Dispatches an event of `type` here and up to the document, as a bubbling DOM event; returns
   * it, so that the caller sees whether a listener prevented what it would do. */
  fire(type, more = {}) {
    const event = {
      type,
      target: this,
      defaultPrevented: false,
      preventDefault() {
        this.defaultPrevented = true;
      },
      stopPropagation() {},
      ...more,
    };
    for (const node of outward(this)) {
      event.currentTarget = node;
      node[`on${type}`]?.(event);
      for (const listener of node.listeners[type] ?? []) listener(event);
    }
    for (const listener of globalThis.document.listeners[type] ?? []) listener(event);
    return event;
  }
  dispatchEvent(event) {
    return !this.fire(event.type).defaultPrevented;
  }
  /** A click as a browser makes it: nothing on a disabled element; a submit button submits its
   * form. Returns whether it clicked. */
  click() {
    if (this.disabled) return false;
    this.fire("click");
    if (this.localName === "button" && this.type === "submit") this.form?.requestSubmit();
    return true;
  }
  requestSubmit() {
    this.fire("submit");
  }
  focus() {
    globalThis.document.activeElement = this;
  }

  matches(selector) {
    return parseList(selector).some((complex) => matchComplex(this, complex));
  }
  closest(selector) {
    return outward(this).find((node) => node.matches(selector)) ?? null;
  }
  descendants() {
    const found = [];
    const walk = (node) => {
      for (const child of node.children) {
        found.push(child);
        walk(child);
      }
    };
    walk(this);
    return found;
  }
  querySelectorAll(selector) {
    const list = parseList(selector);
    return this.descendants().filter((e) => list.some((complex) => matchComplex(e, complex)));
  }
  querySelector(selector) {
    return this.querySelectorAll(selector)[0] ?? null;
  }
}

// ---- selectors: tag, #id, .class, [name], [name=value], :checked, descendants, commas ----

function parseList(selector) {
  return selector.split(",").map((s) => s.trim()).filter(Boolean).map((complex) =>
    complex.split(/\s+/).map(parseCompound)
  );
}

function parseCompound(text) {
  const compound = { tag: null, id: null, classes: [], attributes: [], checked: false };
  const pattern =
    /([a-zA-Z][a-zA-Z0-9-]*|\*)|#([\w-]+)|\.([\w-]+)|\[([\w-]+)(?:=("[^"]*"|'[^']*'|[^\]]*))?\]|(:checked)/g;
  let at = 0;
  for (const match of text.matchAll(pattern)) {
    if (match.index !== at) throw new Error(`selector ${text}`);
    at = match.index + match[0].length;
    if (match[1]) {
      compound.tag = match[1] === "*" ? null : match[1].toLowerCase();
    } else if (match[2]) {
      compound.id = match[2];
    } else if (match[3]) {
      compound.classes.push(match[3]);
    } else if (match[4]) {
      const value = match[5]?.replace(/^["'](.*)["']$/, "$1");
      compound.attributes.push([match[4], value]);
    } else {
      compound.checked = true;
    }
  }
  if (at !== text.length) throw new Error(`selector ${text}`);
  return compound;
}

function matchCompound(element, compound) {
  if (!(element instanceof Element)) return false;
  if (compound.tag && element.localName !== compound.tag) return false;
  if (compound.id && element.id !== compound.id) return false;
  if (compound.classes.some((name) => !element.classes().includes(name))) return false;
  for (const [name, value] of compound.attributes) {
    if (!(name in element.attributes)) return false;
    if (value !== undefined && element.attributes[name] !== value) return false;
  }
  return !compound.checked || element.checked;
}

function matchComplex(element, complex) {
  if (!matchCompound(element, complex.at(-1))) return false;
  let i = complex.length - 2;
  for (let node = element.parentNode; i >= 0 && node instanceof Element; node = node.parentNode) {
    if (matchCompound(node, complex[i])) i--;
  }
  return i < 0;
}

// ---- the parser ----

const entities = (text) =>
  text.replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&quot;/g, '"')
    .replace(/&nbsp;/g, " ").replace(/&#10;/g, "\n").replace(/&amp;/g, "&");

function parse(html) {
  const root = new Element("#root");
  const open = [root];
  let i = 0;
  while (i < html.length) {
    if (html.startsWith("<!--", i)) {
      i = html.indexOf("-->", i) + 3;
    } else if (html.startsWith("<!", i)) {
      i = html.indexOf(">", i) + 1;
    } else if (html.startsWith("</", i)) {
      const end = html.indexOf(">", i);
      const tag = html.slice(i + 2, end).trim().toLowerCase();
      i = end + 1;
      const at = open.map((element) => element.localName).lastIndexOf(tag);
      if (at > 0) open.length = at;
    } else if (html[i] === "<" && /[a-zA-Z]/.test(html[i + 1] ?? "")) {
      const tag = /^[a-zA-Z][a-zA-Z0-9-]*/.exec(html.slice(i + 1))[0].toLowerCase();
      let j = i + 1 + tag.length;
      const attributes = {};
      const pattern = /\s*([^\s=/>]+)(?:\s*=\s*("[^"]*"|'[^']*'|[^\s>]+))?|\s*(\/?>)/y;
      for (;;) {
        pattern.lastIndex = j;
        const match = pattern.exec(html);
        if (!match) throw new Error(`index.html at ${j}: ${html.slice(j, j + 40)}`);
        j = pattern.lastIndex;
        if (match[3]) break;
        attributes[match[1].toLowerCase()] = entities(
          (match[2] ?? "").replace(/^["'](.*)["']$/s, "$1"),
        );
      }
      i = j;
      const element = new Element(tag, attributes);
      open.at(-1).appendChild(element);
      if (RAW.has(tag)) {
        const end = html.indexOf(`</${tag}`, i);
        element.appendChild(new Text(html.slice(i, end)));
        i = html.indexOf(">", end) + 1;
      } else if (!VOID.has(tag)) {
        open.push(element);
      }
    } else {
      const end = html.indexOf("<", i) < 0 ? html.length : html.indexOf("<", i);
      open.at(-1).appendChild(new Text(entities(html.slice(i, end))));
      i = end;
    }
  }
  return root;
}

/** The document of the page `html`. */
export function makeDocument(html) {
  top = parse(html);
  return {
    documentElement: top.querySelector("html"),
    body: top.querySelector("body"),
    activeElement: null,
    listeners: {},
    getElementById: (id) => top.descendants().find((element) => element.id === id) ?? null,
    querySelectorAll: (selector) => top.querySelectorAll(selector),
    querySelector: (selector) => top.querySelector(selector),
    createElement: (tag) => new Element(tag),
    addEventListener(type, listener) {
      (this.listeners[type] ??= []).push(listener);
    },
  };
}

/** `new Option(text, value)` as browsers have it. */
export function Option(text = "", value = undefined) {
  const option = new Element("option", value === undefined ? {} : { value: String(value) });
  option.append(String(text));
  return option;
}
