package pa; localparam P = 3; endpackage
package pb; import pa::P; export pa::P; endpackage
module top;
  import pa::P;
  import pb::P;
  initial #1 $display("d03 P=%0d", P);
  initial #100 $finish;
endmodule
