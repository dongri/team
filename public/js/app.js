
marked.setOptions({
  renderer: new marked.Renderer(),
  gfm: true,
  tables: true,
  breaks: true,
  pedantic: true,
  sanitize: true,
  smartLists: true,
  smartypants: false
});

$(document).ready(function(){
  emojify.setConfig({
    img_dir : '/img/emoji', // Directory for emoji images
  });
  emojify.run();

  $('textarea', '.CodeMirror').textcomplete([
    {
      match: /\B:([\-+\w]*)$/,
      search: function (term, callback) {
        callback($.map(emojies, function (emoji) {
          return emoji.indexOf(term) === 0 ? emoji : null;
        }));
      },
      template: function (value) {
        return '<img class="emoji-suggest" src="/img/emoji/' + value + '.png"></img> ' + value;
      },
      replace: function (value) {
        return ':' + value + ': ';
      },
      index: 1,
      maxCount: 10
    }
  ], {
    onKeydown: function (e, commands) {
      if (e.ctrlKey && e.keyCode === 74) {
        alert('aaa');
        return commands.KEY_ENTER;
      }
    }
  });

  $('#input-comment').textcomplete([
    {
      match: /\B:([\-+\w]*)$/,
      search: function (term, callback) {
        callback($.map(emojies, function (emoji) {
          return emoji.indexOf(term) === 0 ? emoji : null;
        }));
      },
      template: function (value) {
        return '<img class="emoji-suggest" src="/img/emoji/' + value + '.png"></img> ' + value;
      },
      replace: function (value) {
        return ':' + value + ': ';
      },
      index: 1,
      maxCount: 10
    }
  ], {
    onKeydown: function (e, commands) {
      if (e.ctrlKey && e.keyCode === 74) {
          return commands.KEY_ENTER;
      }
    }
  });

});
