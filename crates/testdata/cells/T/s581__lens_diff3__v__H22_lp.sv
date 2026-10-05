module top;
  localparam logic [63:0] P64H = 64'h8000_0000_0000_000C;
  localparam R = ({1'b0, P64H} ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
