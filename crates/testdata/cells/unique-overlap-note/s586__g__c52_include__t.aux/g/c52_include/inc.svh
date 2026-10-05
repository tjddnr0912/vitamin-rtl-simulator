// header
task automatic chk(input logic [1:0] r, output logic y);
  y = 0;
  unique0 casez (r)
    2'b?1: y = 1;
    2'b1?: y = 1;
  endcase
endtask
