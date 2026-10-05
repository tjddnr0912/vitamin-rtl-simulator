typedef logic [3:0] nib_t;
function automatic nib_t f(nib_t x);
  unique case (x) 4'd0: return 4'd1; default: return 4'd2; endcase
endfunction
