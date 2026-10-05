package pk; localparam string i = "AB"; endpackage
module top;
  import pk::*;
  for (genvar i = 0; i < 2; i++) begin : g
    function automatic int f(); return i + 0; endfunction
    initial #1 $display("fn %m f=%0d", f());
  end
  initial #5 $display("post %s", i);
  initial #100 $finish;
endmodule
