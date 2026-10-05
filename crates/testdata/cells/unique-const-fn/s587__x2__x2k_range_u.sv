module top;
  function automatic logic [3:0] fx(input int a);
    unique if (a == 1) fx = 4'd10;
  endfunction
  logic [fx(2):0] v;
  initial begin #1 $display("b=%0d", $bits(v)); $finish; end
endmodule
