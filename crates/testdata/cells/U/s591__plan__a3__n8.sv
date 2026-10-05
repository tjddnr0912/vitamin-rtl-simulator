module top;
  localparam real i = 2.5;
  for (genvar i = 0; i < 2; i++) begin : g
    begin : h
      localparam real i = 1.5;
      initial #2 $display("n8 in-h %m i=%f", i);
    end
    initial #3 $display("n8 body %m i=%0d", i);
  end
  initial #5 $display("n8 post i=%f", i);
  initial #100 $finish;
endmodule
