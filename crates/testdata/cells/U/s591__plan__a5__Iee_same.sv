package pa; localparam P = 3; endpackage
module top;
  import pa::P;
  import pa::P;
  initial #1 $display("ees P=%0d", P);
  initial #100 $finish;
endmodule
