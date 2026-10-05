package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
package pc;
  import pb::P;
  localparam [64:0] Z = P;
  localparam [64:0] Y = P + 65'd1;
  localparam int K = P;
  localparam int B = $bits(P);
endpackage
module top;
  initial #1 $display("pk Z=%0d Y=%0d K=%0d B=%0d", pc::Z, pc::Y, pc::K, pc::B);
  initial #100 $finish;
endmodule
