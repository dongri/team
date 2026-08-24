// Team — application script (vanilla JS)

document.addEventListener('DOMContentLoaded', function () {
  // ----- User dropdown -----
  var dropdown = document.getElementById('user-dropdown');
  if (dropdown) {
    var trigger = dropdown.querySelector('.avatar-btn');
    trigger.addEventListener('click', function (event) {
      event.stopPropagation();
      dropdown.classList.toggle('is-active');
      trigger.setAttribute('aria-expanded', dropdown.classList.contains('is-active'));
    });
    document.addEventListener('click', function () {
      dropdown.classList.remove('is-active');
      trigger.setAttribute('aria-expanded', 'false');
    });
  }

  // ----- Unread notification badge -----
  var bell = document.getElementById('notification-icon');
  if (bell) {
    fetch('/notification_count')
      .then(function (res) { return res.ok ? res.json() : { count: 0 }; })
      .then(function (data) {
        bell.classList.toggle('has-notification', data.count > 0);
      })
      .catch(function () { /* ignore */ });
  }

  // ----- Markdown rendering -----
  var md = window.markdownit
    ? window.markdownit({ html: false, linkify: true, breaks: true })
    : null;
  document.querySelectorAll('.marked').forEach(function (element) {
    if (md) {
      element.innerHTML = md.render(element.textContent);
      element.querySelectorAll('pre code').forEach(function (block) {
        if (window.hljs) {
          if (hljs.highlightElement) {
            hljs.highlightElement(block);
          } else if (hljs.highlightBlock) {
            hljs.highlightBlock(block);
          }
        }
      });
    }
    // Always reveal the content, even if markdown-it failed to load.
    element.hidden = false;
    element.style.display = '';
  });

  // ======================================================
  // Emoji (:emoji:) — rendering + autocomplete
  // ======================================================
  var EMOJI = window.EMOJI_NAMES || [];
  var EMOJI_SET = {};
  EMOJI.forEach(function (n) { EMOJI_SET[n] = true; });
  var EMOJI_RE = /:([a-zA-Z0-9_+-]+):/g;

  function emojiImg(name) {
    var img = document.createElement('img');
    img.className = 'emoji';
    img.src = '/img/emoji/' + encodeURIComponent(name) + '.png';
    img.alt = ':' + name + ':';
    img.title = ':' + name + ':';
    return img;
  }

  // Replace :name: in a text node with <img> (only for known names)
  function emojifyTextNode(node) {
    var text = node.nodeValue;
    var m, last = 0, frag = null;
    EMOJI_RE.lastIndex = 0;
    while ((m = EMOJI_RE.exec(text))) {
      if (!EMOJI_SET[m[1]]) continue;
      frag = frag || document.createDocumentFragment();
      frag.appendChild(document.createTextNode(text.slice(last, m.index)));
      frag.appendChild(emojiImg(m[1]));
      last = m.index + m[0].length;
    }
    if (frag) {
      frag.appendChild(document.createTextNode(text.slice(last)));
      node.parentNode.replaceChild(frag, node);
    }
  }

  function emojifyTree(root) {
    var SKIP = { PRE: 1, CODE: 1, SCRIPT: 1, STYLE: 1, TEXTAREA: 1, INPUT: 1 };
    var walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
      acceptNode: function (node) {
        if (node.nodeValue.indexOf(':') === -1) return NodeFilter.FILTER_SKIP;
        for (var el = node.parentElement; el; el = el.parentElement) {
          if (SKIP[el.tagName] || el.classList.contains('CodeMirror')) {
            return NodeFilter.FILTER_REJECT;
          }
        }
        return NodeFilter.FILTER_ACCEPT;
      },
    });
    var nodes = [];
    while (walker.nextNode()) nodes.push(walker.currentNode);
    nodes.forEach(emojifyTextNode);
  }

  emojifyTree(document.body);

  // ----- Autocomplete popup (shared by textareas and CodeMirror) -----
  var suggestBox = document.createElement('div');
  suggestBox.className = 'emoji-suggest-box';
  suggestBox.hidden = true;
  document.body.appendChild(suggestBox);
  var suggest = null; // { matches, active, insert }

  function hideSuggest() {
    suggestBox.hidden = true;
    suggest = null;
  }

  function renderSuggest() {
    suggestBox.innerHTML = '';
    suggest.matches.forEach(function (name, i) {
      var item = document.createElement('div');
      item.className = 'emoji-suggest-item' + (i === suggest.active ? ' is-active' : '');
      item.appendChild(emojiImg(name));
      item.appendChild(document.createTextNode(':' + name + ':'));
      item.addEventListener('mousedown', function (e) {
        e.preventDefault();
        var insert = suggest.insert;
        hideSuggest();
        insert(name);
      });
      suggestBox.appendChild(item);
    });
  }

  function showSuggest(term, x, y, insert) {
    var matches = EMOJI.filter(function (n) { return n.indexOf(term) === 0; }).slice(0, 8);
    if (!matches.length) return hideSuggest();
    suggest = { matches: matches, active: 0, insert: insert };
    renderSuggest();
    suggestBox.style.left = x + 'px';
    suggestBox.style.top = y + 'px';
    suggestBox.hidden = false;
  }

  // Returns true when the key event was consumed by the popup
  function suggestKeydown(e) {
    if (!suggest) return false;
    if (e.key === 'ArrowDown') {
      suggest.active = (suggest.active + 1) % suggest.matches.length;
      renderSuggest();
    } else if (e.key === 'ArrowUp') {
      suggest.active = (suggest.active - 1 + suggest.matches.length) % suggest.matches.length;
      renderSuggest();
    } else if (e.key === 'Enter' || e.key === 'Tab') {
      var insert = suggest.insert;
      var name = suggest.matches[suggest.active];
      hideSuggest();
      insert(name);
    } else if (e.key === 'Escape') {
      hideSuggest();
    } else {
      return false;
    }
    e.preventDefault();
    e.codemirrorIgnore = true;
    return true;
  }

  var TERM_RE = /\B:([a-zA-Z0-9_+-]{1,30})$/;

  // Plain textareas (comments, tweets)
  document.querySelectorAll('textarea[name="body"]').forEach(function (ta) {
    ta.addEventListener('input', function () {
      var m = TERM_RE.exec(ta.value.slice(0, ta.selectionStart));
      if (!m) return hideSuggest();
      var rect = ta.getBoundingClientRect();
      showSuggest(m[1], rect.left + window.scrollX, rect.bottom + window.scrollY + 4, function (name) {
        var start = ta.selectionStart;
        var before = ta.value.slice(0, start).replace(TERM_RE, ':' + name + ': ');
        ta.value = before + ta.value.slice(start);
        ta.setSelectionRange(before.length, before.length);
        ta.focus();
      });
    });
    ta.addEventListener('keydown', suggestKeydown);
    ta.addEventListener('blur', function () { setTimeout(hideSuggest, 150); });
  });

  // SimpleMDE editor (post / nippo forms)
  if (window.simplemde && window.simplemde.codemirror) {
    var cm = window.simplemde.codemirror;

    function checkEditor() {
      var cur = cm.getCursor();
      var m = TERM_RE.exec(cm.getLine(cur.line).slice(0, cur.ch));
      if (!m) return hideSuggest();
      var coords = cm.cursorCoords(true, 'page');
      showSuggest(m[1], coords.left, coords.bottom + 4, function (name) {
        var to = cm.getCursor();
        var from = { line: to.line, ch: to.ch - m[1].length - 1 };
        cm.replaceRange(':' + name + ': ', from, to);
        cm.focus();
      });
    }

    cm.on('inputRead', checkEditor);
    cm.on('change', function (_cm, change) {
      if (change.origin === '+delete') checkEditor();
    });
    cm.on('keydown', function (_cm, e) { suggestKeydown(e); });
    cm.on('blur', function () { setTimeout(hideSuggest, 150); });
  }
});
