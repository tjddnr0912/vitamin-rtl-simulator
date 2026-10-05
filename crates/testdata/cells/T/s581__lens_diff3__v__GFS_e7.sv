module top;
  localparam logic [64:0] P65 = 65'hC;
  for (genvar i = 0; i < 4; i = i + 1 + (P65 ==? 4'b1?00)) begin : g
    initial $display("G=%0d", i);
  end
  initial #100 $finish;
endmodule
