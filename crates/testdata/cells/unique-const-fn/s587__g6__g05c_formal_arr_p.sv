module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  function automatic int sz(input int a [0:f(2)]);
    return $size(a) * 100 + a[7];
  endfunction
  int arr [0:7];
  initial begin
    foreach (arr[i]) arr[i] = i + 10;
    #1 $display("s=%0d", sz(arr)); $finish;
  end
endmodule
