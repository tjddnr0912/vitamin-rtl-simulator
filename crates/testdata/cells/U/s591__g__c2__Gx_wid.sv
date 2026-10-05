module top;
  for (genvar i = 0; i < 2; i++) begin : g
    logic [i+1:0] v;
    initial #1 $display("wid %m b=%0d", $bits(v));
  end
  initial #100 $finish;
endmodule
