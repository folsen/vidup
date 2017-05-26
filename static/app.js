$(function() {
  $form = $('#upload-area');
  $form.on('drag dragstart dragend dragover dragenter dragleave drop', function(e) {
      e.preventDefault();
      e.stopPropagation();
    })
    .on('dragover dragenter', function() {
      $form.addClass('is-dragover');
    })
    .on('dragleave dragend drop', function() {
      $form.removeClass('is-dragover');
    })
    .on('drop', function(e) {
      droppedFiles = e.originalEvent.dataTransfer.files;
      if (droppedFiles.length > 1)
        $('.msg').html('You can only upload one file at a time');
      upload(droppedFiles[0]);
    });

});

function upload(file){
  //var reader = new FileReader();
  $('#upload-msg').hide();
  $('#uploading-msg').show();
  //reader.onload = function(e) {
    $.ajax({
      xhr: function() {
        var xhr = new window.XMLHttpRequest();
        xhr.upload.addEventListener("progress", function(evt) {
          if (evt.lengthComputable) {
            var percentComplete = evt.loaded / evt.total;
            $('#uploading-msg .percent').html(Math.round(percentComplete * 100) + "%");
          }
       }, false);
       xhr.upload.addEventListener("load", function(evt) {
         $('#uploading-msg').hide();
         $('#processing-msg').show();
       }, false);

       return xhr;
      },
      type: 'POST',
      url: '/upload',
      //data: event.target.result,
      data: file,
      processData: false,
      contentType: false
    }).done(function(data) {
      window.location = "/"+data;
    }).fail(function(data) {
      $('#upload-msg').show();
      $('#processing-msg').hide();
      $('.upload-msg').html('Could not process file, unsupported file format.');
    });
  //};
  //reader.readAsArrayBuffer(file);
}
