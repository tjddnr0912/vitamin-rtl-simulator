`timescale 1ns/1ns
module t;
  localparam logic [3:0] PV = 4'b1100;
  for (genvar i = 0; i < 2; i++) begin : g
    if ((PV + i) ==? 4'b110?) begin : h initial $display("C31q i=%0d in", i); end
    else begin : k initial $display("C31q i=%0d out", i); end
  end
  initial #1 $finish;
  initial #100 $finish;
endmodule
