module top;
  localparam string i = "AB";
  for (genvar i = 0; i < 2; i++) begin : g
    begin : h
      localparam string i = "CD";
      initial #2 $display("n9 in-h %m i=%s", i);
    end
    initial #3 $display("n9 body %m i=%0d", i);
  end
  initial #5 $display("n9 post i=%s", i);
  initial #100 $finish;
endmodule
