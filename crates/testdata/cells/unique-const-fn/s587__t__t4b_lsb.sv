module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  logic [15:0] v = 16'hF0F0;
  initial begin #1 $display("pl=%h", v[11:f(2)]); $finish; end
endmodule
