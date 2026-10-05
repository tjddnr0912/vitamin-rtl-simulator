module top;
  function automatic logic [3:0] fx(input int a);
    if (a == 1) fx = 4'd10;
  endfunction
  localparam logic [3:0] P = fx(2);
  initial begin #1 $display("P=%b", P); $finish; end
endmodule
