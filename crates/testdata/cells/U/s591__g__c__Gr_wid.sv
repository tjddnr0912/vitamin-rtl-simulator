module top;
  localparam real i = 2.5;
  for (genvar i = 0; i < 2; i++) begin : g
    logic [i+1:0] v;
    initial #1 $display("wid %m b=%0d", $bits(v));
  end
  initial #5 $display("post %f", i);
  initial #100 $finish;
endmodule
