"use strict";

/* Define $, $$ and ensure forEach exists */
function $(selector, context) {
    /* Standard element selector */
    context = context || document;
    if (!context.querySelector) {
        return undefined;
    }
    return context.querySelector(selector);
}
function $$(selector, context) {
    /* Standard multiple element selector */
    context = context || document;
    if (!context.querySelector) {
        return undefined;
    }
    return context.querySelectorAll(selector);
}
if (!NodeList.prototype.forEach) {
    /* Internet Explorer and Edge... */
    NodeList.prototype.forEach = function(func) {
        for (var i = 0; i < this.length; i += 1) {
            func(this[i]);
        }
    }
}

/* Show time in locale format in tooltip when hovering <time> elements */
(function(){
    $$('time').forEach(function(element) {
        let iso8601 = element.getAttribute('datetime') || element.innerText
        let datetime = new Date(iso8601);
        let options = {
            year: 'numeric', month: 'long', day: 'numeric', weekday: 'long',
            hour: 'numeric', minute: 'numeric', second: 'numeric',
            timeZoneName: 'short',
        };
        element.title = datetime.toLocaleString(undefined, options);
    });
})();

/* Toggle <rt> in <ruby> elements depending on <input type="radio"> */
(function(){
    let toggles = $('#rubby-toggles');
    $$('input', toggles).forEach(function(element) {
        element.addEventListener('click', update);
    });
    set_mode(localStorage.getItem('ruby-toggle') || 'hover');
    update();

    function set_mode(mode) {
        let input_element = $('[value=' + mode + ']', toggles)
        input_element.checked = true;
    }

    function get_mode(mode) {
        return $(':checked', toggles).value;
    }

    function update(event) {
        let mode = get_mode();
        localStorage.setItem('ruby-toggle', mode);
        document.body.classList.remove('ruby-never');
        document.body.classList.remove('ruby-always');
        if (mode == 'always') {
            document.body.classList.add('ruby-always');
        } else if (mode == 'never') {
            document.body.classList.add('ruby-never');
        }
    }
})();

/* The rest is rikai handling (loading EDICT and showing translations) */
(function(){
let edict = {};
let enamdict = {};
let deinflect = [];
let rikai_container = $('#rikai-container');
let rikai_edict = $('#rikai-edict', rikai_container);
let rikai_names = $('#rikai-names', rikai_container);
let rikai_mask = $('#rikai-mask', rikai_container);
let autorikai_checkbox = $('#rikai_auto');

hide_rikai();

/* Start downloading data */
fetch('/static/nhkstories/deinflect.dat', load_deinflect);
fetch('/media/subedict.dat', load_edict);
fetch('/media/subenamdict.dat', load_enamdict);

/* Event binding */
window.addEventListener('mousemove', autorikai);
window.addEventListener('click', manualrikai);
window.addEventListener('touchstart', disable_autorikai);
rikai_edict.addEventListener('click', ignore_event);
rikai_edict.addEventListener('touchstart', ignore_event);
rikai_names.addEventListener('click', ignore_event);
rikai_names.addEventListener('touchstart', ignore_event);
rikai_mask.addEventListener('click', hide_rikai);

/* Event handlers */
function autorikai(event) {
    if (autorikai_checkbox.checked) {
        set_rikai_from_point(event.clientX, event.clientY);
    }
}
function manualrikai(event) {
    if (event.button === 0) {
        if (set_rikai_from_point(event.clientX, event.clientY)) {
            disable_autorikai();
        }
    }
}
function ignore_event(event) {
    event.stopPropagation();
}
function show_rikai(event) {
    rikai_container.style.visibility = 'visible';
}
function hide_rikai(event) {
    rikai_container.style.visibility = 'hidden';
    if (event) {
        ignore_event(event);
    }
}
function disable_autorikai() {
    autorikai_checkbox.checked = false;
}

function fetch(url, callback) {
    let req = new XMLHttpRequest();
    req.addEventListener('load', function(event) {
        callback(req.response);
    });
    req.open('GET', url);
    req.send();
}

function parse_edict(dst, data) {
    let regex = /^(\S*)\s+(?:\[(.*?)\])?\s*\/(.*)\//gm;
    let match;
    while ((match = regex.exec(data))) {
        let type = 0;
        if (match[3].search('v1'))    { type |=  256; }
        if (match[3].search('v5'))    { type |=  512; }
        if (match[3].search('adj-i')) { type |= 1024; }
        if (match[3].search('vk'))    { type |= 2048; }
        if (match[3].search('vs'))    { type |= 4096; }

        let kanji, kana;
        if (match[2]) {
            kanji = match[1];
            kana = match[2];
        } else {
            kana = match[1];
        }

        let info = {
            'kanji': kanji,
            'kana': kana,
            'glosses': match[3].replace(/\//g, '; '),
            'type': type,
        };
        dst[kana] = info;
        dst[kanji] = info;
    }
}

function load_edict(data) {
    parse_edict(edict, data);
}

function load_enamdict(data) {
    parse_edict(enamdict, data);
}

function load_deinflect(data) {
    let lines = data.split('\n');
    let reasons = [];
    for (let i = 1; i < lines.length; i += 1) {  // skip headers
        let fields = lines[i].split('\t');
        if (fields.length == 1) {  // string array
            reasons.push(lines[i]);
        } else {  // deinflection
            let [from, to, type, reason] = fields;
            reason = reasons[reason];
            deinflect.push([from, to, type, reason]);
        }
    }
}

function get_text_at_point(x, y) {
    let range, target, offset;
    if (document.caretPositionFromPoint) {
        range = document.caretPositionFromPoint(x, y);
        target = range.offsetNode;
        offset = range.offset;
    } else if (document.caretRangeFromPoint) {
        range = document.caretRangeFromPoint(x, y);
        target = range.startContainer;
        offset = range.startOffset;
    }
    if (!target || target.nodeType != Node.TEXT_NODE) {
        return '';
    }
    if (target.parentNode.nodeName == 'LABEL') {
        // label for toggling auto rikai
        return '';
    }
    // check that the cursor is actually within target
    let rect = target.parentNode.getBoundingClientRect();
    if (!rect ||
        !(rect.left <= x && x <= rect.right) ||
        !(rect.top <= y && y <= rect.bottom)) {
        return '';
    }
    // we iterate all the text nodes around the target (pertaining to the
    // same block (non-inline) element), skip to the target, and then
    // ignore ruby annotations (rt)
    let parent = target;
    let display;
    do {
        parent = parent.parentNode;
        display = getComputedStyle(parent).display;
    } while (display == 'inline' || display == 'ruby');
    // treeWalker and nodeIterator are the same unless the DOM is modified
    // <https://mail-archives.apache.org/mod_mbox/xml-general/200012.mbox/%3C002d01c05bbe$42d71680$1000a8c0@equitytg.com%3E>
    let treeWalker = document.createTreeWalker(parent, NodeFilter.SHOW_TEXT);

    while (treeWalker.nextNode() != target);  // skip nodes before target
    let text = target.data.substring(offset);
    // nodes after target
    while (treeWalker.nextNode()) {
        if (treeWalker.currentNode.parentNode.nodeName == 'RT') {
            continue;  // skip ruby annotations
        }
        text += treeWalker.currentNode.data;
    }
    return text;
}

function iter_subfragments(text, callback) {
    let re = /^[\u25cb\u3004-\u30ff\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff\uff66-\uff9f]+/g;
    let match = re.exec(text);
    if (!match) {
        return;
    }
    let fragment = match[0];
    for (var stop = fragment.length+1; stop --> 1; ) {
        callback(fragment.substring(0, stop));
    }
}

function iter_deinflections(word, callback) {
    callback(word, 0, null);
    for (let i = 0; i < deinflect.length; i += 1) {
        let [from, to, type, reason] = deinflect[i];
        if (word.endsWith(from)) {
            let n = word.length - from.length;
            let candidate = word.substring(0, n) + to;
            callback(candidate, type, reason);
        }
    }
}

function append_sense(html, sense, reason) {
    if (sense.kanji) {
        html.push('<dt><a href="https://jisho.org/search/%23kanji%20');
        html.push(sense.kanji);
        html.push('">');
        html.push(sense.kanji);
        html.push('</a></dt><dt>');
    }
    html.push(sense.kana);
    if (reason) {
        html.push(' (');
        html.push(reason);
        html.push(')');
    }
    html.push('</dt><dd>');
    html.push(sense.glosses);
    html.push('</dd>');
}

function set_rikai_from_point(x, y) {
    let text = get_text_at_point(x, y);

    let edict_html = [];
    iter_subfragments(text, function(subfragment) {
        iter_deinflections(subfragment, function(candidate, type, reason) {
            let info = edict[candidate];
            if (!info || (type  && !(info.type & type))) {
                return;
            }
            append_sense(edict_html, info, reason);
        });
    });

    let names_html = [];
    iter_subfragments(text, function(candidate) {
        let info = enamdict[candidate];
        if (info) {
            append_sense(names_html, info);
        }
    });

    if (edict_html.length || names_html.length) {
        $('dl', rikai_edict).innerHTML = edict_html.join('');
        $('dl', rikai_names).innerHTML = names_html.join('');
        show_rikai();
        return true;
    } else {
        hide_rikai();
        return false;
    }
}

})();
