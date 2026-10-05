package pk; localparam [64:0] i = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pk::i;
  for (genvar i = 0; i < 2; i++) begin : g
    function automatic int f(); return i + 0; endfunction
    initial #1 $display("fn %m f=%0d", f());
  end
  initial #5 $display("post %0d", i);
  initial #100 $finish;
endmodule
