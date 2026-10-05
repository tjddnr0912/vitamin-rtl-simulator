module top;
  function automatic int fl(input int n);
    fl = 0;
    for (int i = 0; i < n; i++) begin
      if (i == -1) fl = 100;
      fl = fl + 1;
    end
  endfunction
  localparam int P = fl(1000);
  localparam int Q = fl(30000);
  initial begin #1 $display("P=%0d Q=%0d", P, Q); $finish; end
endmodule
