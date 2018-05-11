
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

  var dropdown = document.querySelector('.dropdown');
  dropdown.addEventListener('click', function (event) {
    event.stopPropagation();
    dropdown.classList.toggle('is-active');
  });

  document.addEventListener('click', function (event) {
    event.stopPropagation();
    dropdown.classList.remove('is-active');
  });

  $.ajax({
    url: '/notification_count',
    type: 'GET',
    dataType: 'json',
  }).then(
  function (data) {
    if (data.count > 0) {
      $('#notification-icon').addClass('has-notification');
    } else {
      $('#notification-icon').removeClass('has-notification');
    }
  },
  function () {
    alert("Error!");
  });

});

function submitUpdateMenu() {
  var menu = [];
  $('.menu').each(function(){
    if ($(this).prop('checked')) {
      menu.push($(this).val());
    }
  });
  $("#menu_param").val(menu);
  $("#preference_menu_form").submit();
};
