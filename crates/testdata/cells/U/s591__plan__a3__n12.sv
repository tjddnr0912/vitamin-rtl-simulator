module top;
  localparam [64:0] i = 65'h1_0000_0000_0000_0009;
  for (genvar i = 0; i < 2; i++) begin : g
    function automatic integer f(); return i; endfunction
    begin : h
      function automatic integer f2(); return i + 10; endfunction
      initial #3 $display("n12 h %m f2=%0d", f2());
    end
    initial #3 $display("n12 body %m f=%0d", f());
  end
  initial #5 $display("n12 post i=%0d", i);
  initial #100 $finish;
endmodule
