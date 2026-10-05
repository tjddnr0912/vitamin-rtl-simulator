module top;
  function automatic longint f(input int a);
    byte b; shortint s; longint l;
    f = b + s + l + 3;
  endfunction
  localparam longint P = f(2);
  initial begin #1 $display("P=%0d", P); $finish; end
  initial #100 $finish;
endmodule
