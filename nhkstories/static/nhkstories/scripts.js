(function(){"use strict";

let edict = {};
let names = {};
let deinflect = [];

function fetch(url, callback) {
    let req = new XMLHttpRequest();
    req.addEventListener('load', function(evt) {
        callback(req.response);
    });
    req.open('GET', url);
    req.send();
}

function ruby_update(evt) {
    let mode = document.querySelector('input[name=ruby_toggle]:checked').value;
    document.body.classList.remove('ruby_never');
    document.body.classList.remove('ruby_always');
    if (mode == 'always') {
        document.body.classList.add('ruby_always');
    } else if (mode == 'never') {
        document.body.classList.add('ruby_never');
    }
    localStorage.setItem('ruby_toggle', mode);
}

function load_edict(dst, data) {
    let regex = /^(\S*)\s+(?:\[(.*?)\])?\s*\/(.*)\//gm
    let match;
    let i = 0;
    while (match = regex.exec(data)) {
        let type = 0;

        if (match[3].search('v1')) {
            type |= 256;
        }
        if (match[3].search('v5')) {
            type |= 512;
        }
        if (match[3].search('adj-i')) {
            type |= 1024;
        }
        if (match[3].search('vk')) {
            type |= 1024;
        }
        if (match[3].search('vs')) {
            type |= 1024;
        }

        let kanji, kana;
        if (match[2]) {
            kanji = match[1];
            kana = match[2];
        } else {
            kana = match[1];
        }

        let info = {
            'kanji': match[1],
            'kana': match[2],
            'glosses': match[3].replace(/\//g, '; '),
            'type': type,
        };
        dst[match[1]] = info;
        if (match[2]) {
            dst[match[2]] = info;
        }
    }
}

function load_deinflect(data) {
    let lines = data.split('\n');
    let reasons = [];
    for (let i = 1; i < lines.length; i += 1) {  // skip headers
        let line = lines[i];
        let fields = line.split('\t');
        if (fields.length == 1) {  // string array
            reasons.push(line);
        } else {  // deinflection
            let [from, to, type, reason] = fields;
            reason = reasons[reason];
            type = +type;
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

    // skip nodes before target
    let node = treeWalker.nextNode();
    while (treeWalker.currentNode != target) {
        treeWalker.nextNode();
    }
    // treeWalker.currentNode == target
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
    let re = /^[\u25cb\u3004-\u30ff\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff\uff66-\uff9f]+/g
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
        html.push('<dt>');
        html.push('<a href="https://jisho.org/search/%23kanji%20');
        html.push(sense.kanji);
        html.push('">');
        html.push(sense.kanji);
        html.push('</a>');
        html.push('</dt><dt>');
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
    if (!text) {
        return false;
    }

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
        let info = names[candidate];
        if (info) {
            append_sense(names_html, info);
        }
    });

    let html = [];
    if (edict_html.length && names_html.length) {
        html = edict_html.concat(['<hr>'], names_html);
    } else if(edict_html.length) {
        html = edict_html;
    } else {
        html = names_html;
    }

    document.querySelector('#rikai dl').innerHTML = html.join('');
    return html != [];
}

function main() {
    document.querySelectorAll('time').forEach(function(e) {
        let dt = new Date(e.getAttribute('datetime') || e.innerText);
        let options = {
            year: 'numeric', month: 'long', day: 'numeric', weekday: 'long',
            hour: 'numeric', minute: 'numeric', second: 'numeric',
            timeZoneName: 'short',
        };
        e.title = dt.toLocaleString(undefined, options);
    });
    document.querySelectorAll('input[name=ruby_toggle]').forEach(function(e) {
        // radio buttons trigger click event even on keyboard selection
        e.addEventListener('click', ruby_update)
    });
    let mode = localStorage.getItem('ruby_toggle') || 'hover';
    document.querySelector('input[name=ruby_toggle][value=' + mode + ']').checked = true;
    ruby_update(null);

    fetch('/static/nhkstories/rikai/deinflect.dat', data => load_deinflect(data));
    fetch('/static/nhkstories/rikai/edict.dat', data => load_edict(edict, data));
    fetch('/static/nhkstories/rikai/names.dat', data => load_edict(names, data));

    let rikai = document.querySelector('#rikai');
    let rikai_checkbox = document.querySelector('#rikai_auto')
    let auto_rikai_enabled = rikai_checkbox.checked;
    rikai_checkbox.addEventListener('input', function(evt) {
        auto_rikai_enabled = rikai_checkbox.checked;
    });
    rikai.addEventListener('click', function(evt) {
        evt.stopPropagation();
    });
    document.querySelector('#mask').addEventListener('click', function(evt) {
        if (touch_enabled) {
            return;
        }
        rikai.style.display = 'none';
    });
    let touch_enabled = false;
    window.addEventListener('touchstart', function(evt) {
        touch_enabled = true;
        auto_rikai_enabled = false;
        rikai_checkbox.checked = false;
        rikai.style.display = 'none';
    });
    window.addEventListener('click', function(evt) {
        if (evt.button != 0) {
            return;
        }
        if (touch_enabled) {
            return;
        }
        if (!set_rikai_from_point(evt.clientX, evt.clientY)) {
            return;
        }
        auto_rikai_enabled = false;
        rikai_checkbox.checked = false;
    });
    window.addEventListener('dblclick', function(evt) {
        if (!set_rikai_from_point(evt.clientX, evt.clientY)) {
            return;
        }
        rikai.style.display = 'block';
    });
    window.addEventListener('mousemove', function(evt) {
        if (!auto_rikai_enabled) {
            return;
        }
        set_rikai_from_point(evt.clientX, evt.clientY);
    });
}
main();

})();
