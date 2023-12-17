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
if (!String.prototype.startsWith) {
    /* Internet Explorer... */
    String.prototype.startsWith = function(search) {
        return this.substring(0, search.length) === search;
    };
}
if (!String.prototype.endsWith) {
    /* Internet Explorer... */
    String.prototype.endsWith = function(search) {
        return this.substring(this.length - search.length) === search;
    };
}

/* Show time in locale format in tooltip when hovering <time> elements */
(function(){
    $$('time').forEach(function(element) {
        const iso8601 = element.getAttribute('datetime') || element.innerText
        const datetime = new Date(iso8601);
        const options = {
            year: 'numeric', month: 'long', day: 'numeric', weekday: 'long',
            hour: 'numeric', minute: 'numeric', second: 'numeric',
            timeZoneName: 'short',
        };
        element.title = datetime.toLocaleString(undefined, options);
    });
})();

/* Toggle <rt> in <ruby> elements depending on <input type="radio"> */
(function(){
    const toggles = $('#rubby-toggles');
    let last_mode = null;
    $$('input', toggles).forEach(function(element) {
        element.addEventListener('click', update);
    });
    set_mode(localStorage.getItem('ruby-toggle') || 'hover');

    document.addEventListener('keyup', mousekey_toggler);
    $('#ruby-toggles-helper .key').addEventListener('click', mousekey_toggler);

    function mousekey_toggler(event) {
        if (event.key !== undefined && event.key != 'f' && event.key != 'F') {
            return;
        }
        if (event.shiftKey) {
            set_mode('hover');
        } else {
            if (get_mode() == 'always') {
                set_mode('never');
            } else {
                set_mode('always');
            }
        }
        event.preventDefault();
    }

    // try to immediately detect touch devices
    if ('ontouchstart' in window) {
        set_touchdevice_helper_message();
    }
    let tap_start = null;
    let triple_tap = false;
    document.addEventListener('touchstart', function(event) {
        set_touchdevice_helper_message();
        // double-finger tap
        if (event.touches.length >= 2) {
            tap_start = new Date().getTime();
            // tripler-finger tap
            triple_tap = event.touches.length >= 3;
        }
    });
    document.addEventListener('touchend', function(event) {
        if (event.touches.length != 0) {
            return;
        }
        const now = new Date().getTime();
        if (now - tap_start > 300) {
            return;
        }
        if (triple_tap) {
            set_mode('hover');
        } else {
            if (get_mode() == 'always') {
                set_mode('never');
            } else {
                set_mode('always');
            }
        }
        tap_start = null;
        event.preventDefault();
    });
    function set_touchdevice_helper_message() {
        $('#ruby-toggles-helper').innerHTML = 'Double-finger tap to toggle furigana';
    }

    function set_mode(mode) {
        last_mode = get_mode();
        const input_element = $('[value=' + mode + ']', toggles)
        input_element.checked = true;
        update();
    }

    function get_mode() {
        return $(':checked', toggles).value;
    }

    function update(event) {
        const mode = get_mode();
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
const edict = {};
const enamdict = {};
const deinflect = [];

function fetch(url, callback) {
    const req = new XMLHttpRequest();
    req.addEventListener('load', function(event) {
        callback(req.response);
    });
    req.open('GET', url);
    req.send();
}

function dict_set_or_append(dict, key, value) {
    dict[key] = dict[key] || [];
    dict[key].push(value);
}

function parse_edict(dst, data) {
    const edict_line_pattern = /^(\S*)\s+(?:\[(.*?)\])?\s*\/(.*)\//gm;
    let match;
    while ((match = edict_line_pattern.exec(data))) {
        const glosses = match[3].replace(/\//g, '; ');

        let type = 1<<7;
        if (glosses.search('v1') >= 0)    { type |= 1<<0; }
        if (glosses.search('v5') >= 0)    { type |= 1<<1; }
        if (glosses.search('adj-i') >= 0) { type |= 1<<2; }
        if (glosses.search('vk') >= 0)    { type |= 1<<3; }
        if (glosses.search('vs') >= 0)    { type |= 1<<4; }

        const common_marker = /\([^)]*\)/gm;

        if (match[2]) {  // kanjis and kanas are given
            const kanjis = match[1].replace(common_marker, '').split(';');
            const kanas = match[2].replace(common_marker, '').split(';');

            kanjis.forEach(function(kanji) {
                dict_set_or_append(dst, kanji, {
                    'kanjis': [kanji],
                    'kanas': kanas,
                    'glosses': glosses,
                    'type': type,
                });
            });

            kanas.forEach(function(kana) {
                dict_set_or_append(dst, kana, {
                    'kanjis': kanjis,
                    'kanas': [kana],
                    'glosses': glosses,
                    'type': type,
                });
            });
        } else {  // only kanas
            const kanas = match[1].replace(common_marker, '').split(';');
            kanas.forEach(function(kana) {
                dict_set_or_append(dst, kana, {
                    'kanjis': [],
                    'kanas': [kana],
                    'glosses': glosses,
                    'type': type,
                });
            });
        }
    }
}

function load_edict(data) {
    parse_edict(edict, data);
}

function load_enamdict(data) {
    parse_edict(enamdict, data);
}

function load_deinflect(data) {
    const lines = data.split('\n');
    const reasons = [];
    for (let i = 1; i < lines.length; i += 1) {  // skip headers
        const fields = lines[i].split('\t');
        if (fields.length == 1) {  // string array
            reasons.push(lines[i]);
        } else {  // deinflection
            const from = fields[0];
            const to = fields[1];
            const type = fields[2];
            let reason = fields[3];
            reason = reasons[reason];
            deinflect.push([from, to, type, reason]);
        }
    }
}

function get_text_at_point(x, y) {
    let range, target, offset;
    if (document.caretPositionFromPoint) {
        // Mozilla
        range = document.caretPositionFromPoint(x, y);
        target = range.offsetNode;
        offset = range.offset;
    } else if (document.caretRangeFromPoint) {
        // Webkit
        range = document.caretRangeFromPoint(x, y);
        target = range.startContainer;
        offset = range.startOffset;
    } else if (document.body.createTextRange) {
        // MSIE
        range = document.body.createTextRange();
        try {
            range.moveToPoint(x, y);
        } catch (e) {
            return '';
        }
        range.select();
        range = window.getSelection().getRangeAt(0);
        target = range.startContainer;
        offset = range.startOffset;
    } else {
        console.log('Browser supports no text range method!');
        return '';
    }

    if (!target || target.nodeType != Node.TEXT_NODE) {
        return '';
    }
    if (target.parentNode.nodeName == 'RT') {
        // furigana
        return '';
    }
    // check that the cursor is actually within target
    const rect = target.parentNode.getBoundingClientRect();
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
    const treeWalker = document.createTreeWalker(parent, NodeFilter.SHOW_TEXT, null, false);

    while (treeWalker.nextNode() != target);  // skip nodes before target
    let text = target.data.substring(offset);
    // nodes after target
    while (treeWalker.nextNode()) {
        if (treeWalker.currentNode.parentNode.nodeName == 'RT') {
            continue;  // skip ruby annotations
        }
        text += treeWalker.currentNode.data;
    }
    return [text, range, target];
}

function iter_subfragments(text, callback) {
    const re = /^[\u25cb\u3004-\u30ff\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff\uff66-\uff9f]+/g;
    const match = re.exec(text);
    if (!match) {
        return;
    }
    const fragment = match[0];
    for (var stop = fragment.length+1; stop --> 1; ) {
        callback(fragment.substring(0, stop));
    }
}

function iter_deinflections(word, callback) {
    const candidates = [[word, 0xff, []]]
    // consider all candidates and their deinflections recursively
    for (let i = 0; i < candidates.length; i += 1) {
        const candidate = candidates[i];
        const word = candidate[0];
        const wtype = candidate[1];
        const wreason = candidate[2];

        callback(word, wtype, wreason);

        // iterate over rules
        for (let j = 0; j < deinflect.length; j += 1) {
            const rule = deinflect[j];
            const rfrom = rule[0];
            const rto = rule[1];
            const rtype = rule[2];
            const rreason = rule[3];

            // check types match
            if (wtype & rtype === 0) {
                continue;
            }
            // check suffix matches
            if (!word.endsWith(rfrom)) {
                continue
            }

            // append new candidate
            const new_word = word.substr(0, word.length-rfrom.length) + rto;  // replace suffix
            const new_type = rtype >> 8;
            const new_reason = wreason.slice();
            new_reason.push(rreason);
            candidates.push([new_word, new_type, new_reason])
            /* NOTE: could check that new_word is already in candidates
             * Rikaikun merges with previous candidate; if this candidate
             * has already been processed, the new type is ignored
             * Rikaichamp only combines candidates of identical types
             */
        }
    }
}

function append_meaning(html, sense, reason) {
    html.push('<dt>')
    sense.kanjis.forEach(function(kanji) {
        html.push('<span class="kanji">');
        html.push(kanji);
        html.push('</span>');
    });
    sense.kanas.forEach(function(kana) {
        html.push('<span class="kana">');
        html.push(kana);
        html.push('</span>');
    });
    html.push('</dt>');
    if (reason && reason.length) {
        html.push(' (');
        html.push(reason.join(' '));
        html.push(')');
    }
    html.push('<dd>');

    // extract meanings
    const meanings = [];
    let last_meaning = [];
    sense.glosses.split('; ').forEach(function(gloss) {
        if (gloss == '(P)' || gloss.startsWith('EntL')) {
            return;
        }

        const match = /^(?:\(([^0-9]\S?)\) )?(?:\(([0-9]+)\) )?(?:\(.*?\) )*(.*)/.exec(gloss);
        const nature = match[1];
        const meaning_id = match[2];
        const meaning = match[3];
        if (meaning_id !== undefined && last_meaning.length !== 0) {
            meanings.push(last_meaning.join('; '));
            last_meaning = [];
        }
        last_meaning.push(meaning)
    });
    meanings.push(last_meaning.join('; '));

    // list meanings
    html.push('<ol>');
    meanings.forEach(function(meaning) {
        html.push('<li>');
        html.push(meaning);
        html.push('</li>');
    })
    html.push('</ol>');

    html.push('</dd>');
}

function rikai_html(text) {
    const edict_html = [];
    const added_words = [];
    iter_subfragments(text, function(subfragment) {
        iter_deinflections(subfragment, function(candidate, type, reason) {
            const infos = edict[candidate] || [];
            infos.filter(function(info) { return (info.type & type) !== 0; })
            .forEach(function(info) {
                if (added_words.indexOf(info) >= 0) {
                    return;
                }
                added_words.push(info);
                append_meaning(edict_html, info, reason);
            });
        });
    });

    if (edict_html.length) {
        return '<dl>' + edict_html.join('') + '</dl>';
    }

    const names_html = [];
    iter_subfragments(text, function(candidate) {
        const infos = enamdict[candidate] || [];
        infos.forEach(function(info) { return append_meaning(names_html, info); });
    });

    if (names_html.length) {
        return '<dl>' + names_html.join('') + '</dl>';
    }

    return '';
}

function main() {
    if (edict_filename == '.dat') {
        return;
    }

    /* Start downloading data */
    fetch('/static/deinflect.dat', load_deinflect);
    fetch('/media/subedict/' + edict_filename, load_edict);
    fetch('/media/subenamdict/' + edict_filename, load_enamdict);

    const rikai = document.createElement('div');
    rikai.id = 'rikai';
    document.body.appendChild(rikai);

    let last_click = 0;
    function update_rikai(cursorX, cursorY) {
        const now = new Date().getTime();
        if (now - last_click < 1000) {  // 1 second
            return;
        }

        const r = get_text_at_point(cursorX, cursorY);
        const text = r[0];
        const range = r[1];
        const target = r[2];

        const html = rikai_html(text);
        if (html.length == 0) {
            rikai.style.display = 'none';
        } else {
            const lineheight = 39;
            const vertical_margin = 5;

            rikai.innerHTML = html;
            rikai.style.display = 'block';
            let rect;
            if (range.getClientRect !== undefined) {  // Mozilla
                rect = range.getClientRect();
            } else {  // Webkit
                rect = range.getClientRects()[0];
            }
            let x = rect.left;
            let y = rect.bottom + vertical_margin;

            // fix edge case where caret is at the end of previous line
            // we detect this by comparing to the cursor position
            const dx = x - cursorX;
            const dy = y - cursorY;
            const d2 = dx*dx + dy*dy;
            if (d2 > 10000) {  // more than 100px away
                x = target.parentNode.getBoundingClientRect().left;
                y += lineheight;
            }

            // avoid overflow
            rect = rikai.getBoundingClientRect();
            // avoid right overflow
            x = Math.max(0, Math.min(x, document.documentElement.clientWidth - rect.width));
            // avoid bottom overflow
            if (y + rect.height > document.documentElement.clientHeight) {
                const offset = 2*vertical_margin + lineheight + rect.height;
                if (y - offset > 0) {  // no point in clipping the top
                    y -= offset;
                }
            }

            if (window.scrollX === undefined) {  // MSIE
                x += document.documentElement.scrollLeft;
                y += document.documentElement.scrollTop;
            } else {  // necessary for Microsoft Edge
                x += scrollX;
                y += scrollY;
            }
            rikai.style.left = x + 'px';
            rikai.style.top = y + 'px';
        }
    }

    function on_mousemove(event) {
        update_rikai(event.clientX, event.clientY);
    }

    function on_click(event) {
        last_click = new Date().getTime();
        update_rikai(event.clientX, event.clientY);
    }

    function ignore_event(event) {
        event.stopPropagation();
    }

    /* Event binding */
    window.addEventListener('mousemove', on_mousemove);
    window.addEventListener('click', on_click);
    rikai.addEventListener('mousemove', ignore_event);
    rikai.addEventListener('click', ignore_event);
}

main();
})();
