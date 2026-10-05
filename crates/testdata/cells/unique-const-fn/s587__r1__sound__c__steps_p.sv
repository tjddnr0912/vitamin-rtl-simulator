module top;
  function automatic int f(input int n);
    int acc;
    acc = 0;
    for (int i = 0; i < n; i++) begin
      if (i < 0) acc = acc + 100;
      acc = acc + 1;
    end
    return acc;
  endfunction
  localparam int P = f(22000);
  initial begin $display("P=%0d", P); #1 $finish; end
endmodule
