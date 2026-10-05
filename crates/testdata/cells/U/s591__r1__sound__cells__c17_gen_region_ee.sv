package pa; localparam P = 3; endpackage
package pb; localparam P = 5; endpackage
module top;
  import pa::P;
  generate import pb::P; endgenerate
  initial #1 $display("gr P=%0d", P);
  initial #100 $finish;
endmodule
