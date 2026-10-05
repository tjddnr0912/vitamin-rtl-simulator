module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  logic [31:0] v = 32'h0000_0F00;
  initial begin #1 $display("pl=%h", v[11:f(2)]); $finish; end
endmodule
