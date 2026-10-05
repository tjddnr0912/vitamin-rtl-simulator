package pb; localparam P = 3; endpackage
module top;
  import pb::P;
  initial #1 $display("cmp gt=%0d eq3=%0d", (P > 65'd100), (P == 3));
  initial #100 $finish;
endmodule
