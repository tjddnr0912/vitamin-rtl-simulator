module top;
  function automatic int unsigned h(input logic [7:0] a);
    h = 32'hffffffff;
  endfunction
  localparam longint P = h(0);
  initial begin #1 $display("P=%0d", P); $finish; end
  initial #50 $finish;
endmodule
