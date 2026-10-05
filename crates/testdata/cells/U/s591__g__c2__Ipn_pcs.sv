package pb; localparam P = 3; endpackage
module top;
  import pb::P;
  initial #1 case (P) 65'h1_0000_0000_0000_0009: $display("pcs wide"); 3: $display("pcs three"); default: $display("pcs def"); endcase
  initial #100 $finish;
endmodule
