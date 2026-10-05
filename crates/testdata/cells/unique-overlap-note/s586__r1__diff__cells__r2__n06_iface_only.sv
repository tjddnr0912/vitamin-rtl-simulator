interface ifc;
  logic [1:0] r = 2'b11; int y;
  always_comb unique casez (r) 2'b?1: y = 1; 2'b1?: y = 2; default: y = 0; endcase
endinterface
