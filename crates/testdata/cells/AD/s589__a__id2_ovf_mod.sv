module top;
  function automatic logic [31:0] h(input int a);
    h = ((0-1)*(0-1)-2)/2;
  endfunction
  localparam logic [31:0] P = h(0);
  initial begin #1 $display("P=%h", P); $finish; end
  initial #50 $finish;
endmodule
