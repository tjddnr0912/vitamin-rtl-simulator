module top;
  localparam [11:4] i = 8'hA5;
  for (genvar i = 0; i < 2; i++) begin : g
    function automatic int f(); return i + 0; endfunction
    initial #1 $display("fn %m f=%0d", f());
  end
  initial #3 $display("post sel=%h", i[11:8]);
  initial #100 $finish;
endmodule
