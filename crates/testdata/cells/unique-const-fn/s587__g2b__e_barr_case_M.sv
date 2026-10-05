module top;
  function automatic int f(input int a);
    f = 7;
    unique case (a) 1: f = 10; 3: f = 30; endcase
  endfunction
  initial begin : b
    int arr [f(2)];
    #1 $display("s=%0d", $size(arr)); $finish;
  end
endmodule
