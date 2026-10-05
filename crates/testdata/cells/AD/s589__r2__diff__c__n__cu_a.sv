package p;
  localparam int K = 8;
  function automatic int g(); return K; endfunction
  function automatic logic [g()-1:0] f(); return '1; endfunction
endpackage
